mod listen_and_connect;
mod stdio;

use clap::Parser;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// yamux
/// Examples: `yamux localhost 80`, `yamux -l 8080`
#[derive(clap::Parser, Debug)]
#[clap(name = "yamux")]
#[clap(about, version)]
#[clap(global_setting(clap::AppSettings::DeriveDisplayOrder))]
struct Args {
    /// listens
    #[clap(name = "listen", long, short = 'l')]
    listen: bool,

    /// uses Unix-domain socket
    #[clap(name = "unixsock", short = 'U')]
    unixsock: bool,

    /// UDP
    #[clap(name = "udp", short = 'u')]
    udp: bool,

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

    // TODO: handle properly
    if args.udp {
        return run_udp_yamux_client().await;
    }

    if args.listen {
        let listener: listen_and_connect::Listener;
        if args.unixsock {
            if args.rest_args.len() != 1 {
                return Err(anyhow::Error::msg("Unix domain socket is missing"));
            }
            cfg_if::cfg_if! {
                if #[cfg(unix)] {
                    listener = listen_and_connect::Listener::Unix(
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
            listener = listen_and_connect::Listener::Tcp(
                tokio::net::TcpListener::bind((host, port)).await?,
            );
        }
        return run_tcp_yamux_client(listener).await;
    }

    let connector: listen_and_connect::Connector;
    if args.unixsock {
        if args.rest_args.len() != 1 {
            return Err(anyhow::Error::msg("Unix domain socket is missing"));
        }
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                connector = listen_and_connect::Connector::Unix {path: &args.rest_args[0]};
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
        connector = listen_and_connect::Connector::Tcp { host, port };
    }
    return run_tcp_yamux_server(connector).await;
}

async fn run_tcp_yamux_server<'a>(
    connector: listen_and_connect::Connector<'a>,
) -> anyhow::Result<()> {
    use futures::TryStreamExt;

    let yamux_config = yamux::Config::default();
    let yamux_connection =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_connection)
        .try_for_each_concurrent(None, |yamux_stream| {
            let connector = connector.clone();
            async move {
                let (yamux_stream_read, yamux_stream_write) = {
                    use futures::AsyncReadExt;
                    yamux_stream.split()
                };
                let stream_read_write_result = connector.clone().connect().await;
                if let Err(err) = stream_read_write_result {
                    match connector {
                        listen_and_connect::Connector::Tcp { host, port } => {
                            log::warn!("failed to connect {:}:{:}: {:}", host, port, err)
                        }
                        #[cfg(unix)]
                        listen_and_connect::Connector::Unix { path } => {
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

async fn run_tcp_yamux_client(listener: listen_and_connect::Listener) -> anyhow::Result<()> {
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

const UDP_BYTES_LEN: usize = 4;

async fn run_udp_yamux_client() -> anyhow::Result<()> {
    let yamux_config = yamux::Config::default();
    let yamux_client =
        yamux::Connection::new(stdio::Stdio::new(), yamux_config, yamux::Mode::Client);
    let yamux_control = yamux_client.control();

    let yamux_stream = yamux::into_stream(yamux_client);
    tokio::task::spawn({
        use futures::StreamExt;
        yamux_stream.for_each(|_| async {})
    });

    // TODO: hard code
    let udp_socket = Arc::new(tokio::net::UdpSocket::bind(("0.0.0.0", 9443)).await?);
    let mut buf = [0u8; 65536];
    let addr_to_yamux_stream_write: Arc<
        RwLock<
            HashMap<
                std::net::SocketAddr,
                Arc<tokio::sync::Mutex<futures::io::WriteHalf<yamux::Stream>>>,
            >,
        >,
    > = Arc::new(RwLock::new(HashMap::new()));
    loop {
        let (len, addr) = udp_socket.recv_from(&mut buf[UDP_BYTES_LEN..]).await?;
        let addr_to_yamux_stream = addr_to_yamux_stream_write.clone();
        let mut yamux_control = yamux_control.clone();
        let udp_socket = udp_socket.clone();
        tokio::task::spawn(async move {
            let yamux_stream_write_option =
                addr_to_yamux_stream.read().unwrap().get(&addr).cloned();
            let yamux_stream_write = if let Some(yamux_stream_write) = yamux_stream_write_option {
                yamux_stream_write
            } else {
                let yamux_stream_result = yamux_control.open_stream().await;
                if let Err(err) = yamux_stream_result {
                    log::error!("failed to open stream: {:}", err);
                    return;
                }
                let (mut yamux_stream_read, yamux_stream_write) = {
                    use futures::AsyncReadExt;
                    let yamux_stream = yamux_stream_result.unwrap();
                    yamux_stream.split()
                };

                let yamux_stream_write_mutex_arc =
                    Arc::new(tokio::sync::Mutex::new(yamux_stream_write));

                // TODO: expire
                addr_to_yamux_stream
                    .write()
                    .unwrap()
                    .insert(addr, yamux_stream_write_mutex_arc.clone());

                tokio::task::spawn(async move {
                    use futures::AsyncReadExt;
                    let mut buf = [0u8; 65536];
                    while let Ok(_) = yamux_stream_read
                        .read_exact(&mut buf[..UDP_BYTES_LEN])
                        .await
                    {
                        let l: usize =
                            u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                        if let Err(_) = yamux_stream_read.read_exact(&mut buf[..l]).await {
                            return;
                        }
                        if let Err(_) = udp_socket.send_to(&buf[..l], addr.clone()).await {
                            return;
                        }
                    }
                });

                yamux_stream_write_mutex_arc
            };

            {
                use futures::AsyncWriteExt;
                use std::io::Write;
                buf.as_mut().write_all(&(len as u32).to_be_bytes()).unwrap();
                let mut guard = yamux_stream_write.lock().await;
                guard.write_all(&buf[..UDP_BYTES_LEN + len]).await.unwrap();
            }
        });
    }
}
