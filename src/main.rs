mod stdio;

use clap::Parser;

/// yamux
/// Examples: `yamux localhost 80`, `yamux -l 8080`
#[derive(clap::Parser, Debug)]
#[clap(name = "yamux")]
#[clap(about, version)]
#[clap(global_setting(clap::AppSettings::DeriveDisplayOrder))]
struct Args {
    /// listens
    #[clap(long, short = 'l')]
    listen: bool,

    /// uses Unix-domain socket
    #[clap(short = 'U')]
    unixsock: bool,

    /// arguments
    #[clap(name = "ARGUMENTS")]
    rest_args: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse arguments
    let args = Args::parse();

    // Set default log level
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if args.listen {
        let listener: Listener;
        if args.unixsock {
            if args.rest_args.len() != 1 {
                return Err(anyhow::Error::msg("unix domain socket is missing"));
            }
            listener = Listener::UnixListener(tokio::net::UnixListener::bind(&args.rest_args[0])?);
        } else {
            let mut host: &str = "0.0.0.0";
            let port: u16;
            if args.rest_args.len() == 2 {
                host = &args.rest_args[0];
                port = args.rest_args[1].parse()?;
            } else if args.rest_args.len() == 1 {
                port = args.rest_args[0].parse()?;
            } else {
                return Err(anyhow::Error::msg("port number is missing"));
            }
            listener = Listener::TcpListener(tokio::net::TcpListener::bind((host, port)).await?);
        }
        return run_yamux_client(listener).await;
    }

    if args.rest_args.len() != 2 {
        return Err(anyhow::Error::msg("host and port number are missing"));
    }
    // NOTE: should not use std::net::IpAddr because "localhost" could not be the type
    let host: &str = &args.rest_args[0];
    let port: u16 = args.rest_args[1].parse()?;

    return run_yamux_server(host, port).await;
}

async fn run_yamux_server(host: &str, port: u16) -> anyhow::Result<()> {
    use futures::TryStreamExt;

    let yamux_config = yamux::Config::default();
    let yamux_connection =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_connection)
        .try_for_each_concurrent(None, |yamux_stream| async move {
            let (yamux_stream_read, yamux_stream_write) = {
                use futures::AsyncReadExt;
                yamux_stream.split()
            };
            let tcp_stream_result = tokio::net::TcpStream::connect((host, port)).await;
            if let Err(err) = tcp_stream_result {
                log::warn!("failed to connect {:}:{:}: {:}", host, port, err);
                return Ok(());
            }
            let (mut tcp_stream_read, mut tcp_stream_write) =
                tcp_stream_result.unwrap().into_split();
            let fut1 = async move {
                use tokio_util::compat::FuturesAsyncReadCompatExt;
                tokio::io::copy(&mut yamux_stream_read.compat(), &mut tcp_stream_write)
                    .await
                    .unwrap();
            };
            let fut2 = async move {
                use tokio_util::compat::FuturesAsyncWriteCompatExt;
                tokio::io::copy(&mut tcp_stream_read, &mut yamux_stream_write.compat_write())
                    .await
                    .unwrap();
            };
            futures::future::join(fut1, fut2).await;
            Ok(())
        })
        .await?;
    Ok(())
}

enum Listener {
    TcpListener(tokio::net::TcpListener),
    UnixListener(tokio::net::UnixListener),
}

#[auto_enums::enum_derive(tokio1::AsyncRead)]
enum TcpOrUnixAsyncRead {
    TcpAsyncRead(tokio::net::tcp::OwnedReadHalf),
    UnixAsyncRead(tokio::net::unix::OwnedReadHalf),
}

#[auto_enums::enum_derive(tokio1::AsyncWrite)]
enum TcpOrUnixAsyncWrite {
    TcpAsyncWrite(tokio::net::tcp::OwnedWriteHalf),
    UnixAsyncWrite(tokio::net::unix::OwnedWriteHalf),
}

impl Listener {
    fn poll_accept(
        &self,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<tokio::io::Result<(TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite)>> {
        match self {
            Listener::TcpListener(tcp_listener) => {
                let poll = tcp_listener.poll_accept(cx);
                poll.map_ok(|(tcp_stream, _)| {
                    let (r, w) = tcp_stream.into_split();
                    (
                        TcpOrUnixAsyncRead::TcpAsyncRead(r),
                        TcpOrUnixAsyncWrite::TcpAsyncWrite(w),
                    )
                })
            }
            Listener::UnixListener(unix_listener) => {
                let poll = unix_listener.poll_accept(cx);
                poll.map_ok(|(tcp_stream, _)| {
                    let (r, w) = tcp_stream.into_split();
                    (
                        TcpOrUnixAsyncRead::UnixAsyncRead(r),
                        TcpOrUnixAsyncWrite::UnixAsyncWrite(w),
                    )
                })
            }
        }
    }
}

async fn run_yamux_client(listener: Listener) -> anyhow::Result<()> {
    let yamux_config = yamux::Config::default();
    let yamux_client =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Client);
    let yamux_control = yamux_client.control();

    let yamux_stream = yamux::into_stream(yamux_client);
    tokio::task::spawn({
        use futures::StreamExt;
        yamux_stream.for_each(|_| async {})
    });

    loop {
        let (mut listener_read, mut listener_write) =
            futures::future::poll_fn(|cx| listener.poll_accept(cx)).await?;

        let mut yamux_control = yamux_control.clone();
        tokio::task::spawn(async move {
            let yamux_stream_result = yamux_control.open_stream().await;
            if let Err(err) = yamux_stream_result {
                log::error!("failed to open stream: {:}", err);
                return;
            }
            let (yamux_stream_read, yamux_stream_write) = {
                use futures::AsyncReadExt;
                yamux_stream_result.unwrap().split()
            };
            let fut1 = async move {
                use tokio_util::compat::FuturesAsyncReadCompatExt;
                tokio::io::copy(&mut yamux_stream_read.compat(), &mut listener_write)
                    .await
                    .unwrap();
            };
            let fut2 = async move {
                use tokio_util::compat::FuturesAsyncWriteCompatExt;
                tokio::io::copy(&mut listener_read, &mut yamux_stream_write.compat_write())
                    .await
                    .unwrap();
            };
            futures::future::join(fut1, fut2).await;
        });
    }
}
