mod stdio;

use anyhow::Result;
use clap::Parser;

// TODO: better usage
/// yamux
#[derive(clap::Parser, Debug)]
#[clap(name = "yamux")]
#[clap(about, version)]
#[clap(global_setting(clap::AppSettings::DeriveDisplayOrder))]
struct Args {
    /// listens
    #[clap(long, short = 'l')]
    listen: bool,

    /// arguments
    #[clap(group = "input")]
    rest_args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args = Args::parse();

    if args.listen {
        let mut host: &str = "0.0.0.0";
        let mut port: u16 = 0;
        if args.rest_args.len() == 2 {
            host = &args.rest_args[0];
            port = args.rest_args[1].parse()?;
        } else if args.rest_args.len() == 1 {
            port = args.rest_args[0].parse()?;
        } else {
            return Err(anyhow::Error::msg("port number is missing"));
        }
        return run_yamux_client(host, port).await;
    }

    if args.rest_args.len() != 2 {
        return Err(anyhow::Error::msg("host and port number are missing"));
    }
    // NOTE: should not use std::net::IpAddr because "localhost" could not be the type
    let host: &str = &args.rest_args[0];
    let port: u16 = args.rest_args[1].parse()?;

    return run_yamux_server(host, port).await;
}

async fn run_yamux_server(host: &str, port: u16) -> Result<()> {
    use futures::TryStreamExt;

    let yamux_config = yamux::Config::default();
    let yamux_server =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_server)
        .try_for_each_concurrent(None, |yamux_stream| async move {
            let (yamux_stream_read, yamux_write) = {
                use futures::AsyncReadExt;
                yamux_stream.split()
            };
            let (mut connection_read, mut connection_write) =
                tokio::net::TcpStream::connect((host, port))
                    .await?
                    .into_split();
            let fut1 = async move {
                use tokio_util::compat::FuturesAsyncReadCompatExt;
                tokio::io::copy(&mut yamux_stream_read.compat(), &mut connection_write)
                    .await
                    .unwrap();
            };
            let fut2 = async move {
                use tokio_util::compat::FuturesAsyncWriteCompatExt;
                tokio::io::copy(&mut connection_read, &mut yamux_write.compat_write())
                    .await
                    .unwrap();
            };
            futures::future::join(fut1, fut2).await;
            Ok(())
        })
        .await?;
    Ok(())
}

async fn run_yamux_client(host: &str, port: u16) -> Result<()> {
    let yamux_config = yamux::Config::default();
    let yamux_client =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Client);
    let yamux_control = yamux_client.control();

    let yamux_stream = yamux::into_stream(yamux_client);
    tokio::task::spawn({
        use futures::StreamExt;
        yamux_stream.for_each(|_| async {})
    });

    let tcp_listener = tokio::net::TcpListener::bind((host, port)).await?;

    loop {
        let (socket, _) = tcp_listener.accept().await?;
        let (mut socket_read, mut socket_write) = socket.into_split();

        let mut yamux_control = yamux_control.clone();
        tokio::task::spawn(async move {
            let yamux_stream_result = yamux_control.open_stream().await;
            if let Err(err) = yamux_stream_result {
                // TODO: logging
                eprintln!("failed to open: {:?}", err);
                return;
            }
            let (yamux_stream_read, yamux_stream_write) = {
                use futures::AsyncReadExt;
                yamux_stream_result.unwrap().split()
            };

            let fut1 = async move {
                use tokio_util::compat::FuturesAsyncReadCompatExt;
                tokio::io::copy(&mut yamux_stream_read.compat(), &mut socket_write)
                    .await
                    .unwrap();
            };
            let fut2 = async move {
                use tokio_util::compat::FuturesAsyncWriteCompatExt;
                tokio::io::copy(&mut socket_read, &mut yamux_stream_write.compat_write())
                    .await
                    .unwrap();
            };
            futures::future::join(fut1, fut2).await;
        });
    }
}
