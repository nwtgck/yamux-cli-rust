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
        todo!();
        return Err(anyhow::Error::msg("listen not implemented yet"));
    }

    if args.rest_args.len() != 2 {
        return Err(anyhow::Error::msg("host and port number are missing"));
    }

    // NOTE: should not use std::net::IpAddr because "localhost" could not be the type
    let host: &str = &args.rest_args[0];
    let port: u16 = args.rest_args[1].parse()?;

    serve(host, port).await.unwrap();
    Ok(())
}

async fn serve(host: &str, port: u16) -> Result<()> {
    use futures::{AsyncReadExt, TryStreamExt};

    let stdio = stdio::Stdio::new();
    let yamux_config = yamux::Config::default();
    let yamux_server = yamux::Connection::new(stdio, yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_server)
        .try_for_each_concurrent(None, |yamux_stream| async move {
            let (yamux_stream_read, yamux_write) = yamux_stream.split();
            let (mut connection_read, mut connection_write) =
                tokio::net::TcpStream::connect((host, port))
                    .await?
                    .into_split();
            let f1 = async move {
                use tokio_util::compat::FuturesAsyncReadCompatExt;
                tokio::io::copy(&mut yamux_stream_read.compat(), &mut connection_write)
                    .await
                    .unwrap();
            };
            let f2 = async move {
                use tokio_util::compat::FuturesAsyncWriteCompatExt;
                tokio::io::copy(&mut connection_read, &mut yamux_write.compat_write())
                    .await
                    .unwrap();
            };
            futures::future::join(f1, f2).await;
            Ok(())
        })
        .await?;
    Ok(())
}
