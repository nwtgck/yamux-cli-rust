mod listen_and_connect;
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
        let listener: listen_and_connect::Listener;
        if args.unixsock {
            if args.rest_args.len() != 1 {
                return Err(anyhow::Error::msg("Unix domain socket is missing"));
            }
            cfg_if::cfg_if! {
                if #[cfg(unix)] {
                    listener = listen_and_connect::Listener::UnixListener(
                        tokio::net::UnixListener::bind(&args.rest_args[0])?,
                    );
                } else {
                    return Err(anyhow::Error::msg("unix domain socket not supported"));
                }
            }
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
            listener = listen_and_connect::Listener::TcpListener(
                tokio::net::TcpListener::bind((host, port)).await?,
            );
        }
        return run_yamux_client(listener).await;
    }

    let connect_setting: listen_and_connect::ConnectSetting;
    if args.unixsock {
        if args.rest_args.len() != 1 {
            return Err(anyhow::Error::msg("Unix domain socket is missing"));
        }
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                connect_setting = listen_and_connect::ConnectSetting::UnixConnectSetting {
                    path: args.rest_args[0].to_string(),
                };
            } else {
                return Err(anyhow::Error::msg("unix domain socket not supported"));
            }
        }
    } else {
        if args.rest_args.len() != 2 {
            return Err(anyhow::Error::msg("host and port number are missing"));
        }
        // NOTE: should not use std::net::IpAddr because "localhost" could not be the type
        let host: &str = &args.rest_args[0];
        let port: u16 = args.rest_args[1].parse()?;

        connect_setting = listen_and_connect::ConnectSetting::TcpConnectSetting {
            host: host.to_string(),
            port,
        };
    }
    return run_yamux_server(connect_setting).await;
}

async fn run_yamux_server(
    connect_setting: listen_and_connect::ConnectSetting,
) -> anyhow::Result<()> {
    use futures::TryStreamExt;

    let yamux_config = yamux::Config::default();
    let yamux_connection =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_connection)
        .try_for_each_concurrent(None, |yamux_stream| {
            let connect_setting = connect_setting.clone();
            async move {
                let (yamux_stream_read, yamux_stream_write) = {
                    use futures::AsyncReadExt;
                    yamux_stream.split()
                };
                let stream_read_write_result = connect_setting.connect().await;
                if let Err(err) = stream_read_write_result {
                    match connect_setting {
                        listen_and_connect::ConnectSetting::TcpConnectSetting { host, port } => {
                            log::warn!("failed to connect {:}:{:}: {:}", host, port, err)
                        }
                        #[cfg(unix)]
                        listen_and_connect::ConnectSetting::UnixConnectSetting { path } => {
                            log::warn!("failed to connect {:}: {:}", path, err)
                        }
                    }
                    return Ok(());
                }
                let (mut stream_read, mut stream_write) = stream_read_write_result.unwrap();
                let fut1 = async move {
                    use tokio_util::compat::FuturesAsyncReadCompatExt;
                    tokio::io::copy(&mut yamux_stream_read.compat(), &mut stream_write)
                        .await
                        .unwrap();
                };
                let fut2 = async move {
                    use tokio_util::compat::FuturesAsyncWriteCompatExt;
                    tokio::io::copy(&mut stream_read, &mut yamux_stream_write.compat_write())
                        .await
                        .unwrap();
                };
                futures::future::join(fut1, fut2).await;
                Ok(())
            }
        })
        .await?;
    Ok(())
}

async fn run_yamux_client(listener: listen_and_connect::Listener) -> anyhow::Result<()> {
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
        let (mut listener_read, mut listener_write) = listener.accept().await?;
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
