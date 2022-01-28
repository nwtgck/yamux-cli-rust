mod stdio;

use anyhow::Result;
use futures::{AsyncReadExt, TryStreamExt};

#[tokio::main]
async fn main() {
    eprintln!("start");
    serve().await.unwrap();
}

async fn serve() -> Result<()> {
    let stdio = stdio::Stdio {};
    let yamux_config = yamux::Config::default();
    let yamux_server = yamux::Connection::new(stdio, yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_server)
        .try_for_each_concurrent(None, |yamux_stream| async move {
            let (yamux_stream_read, mut yamux_write) = yamux_stream.split();
            let (connection_read, mut connection_write) = {
                use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

                // TODO: hard code
                let (r, w) = tokio::net::TcpStream::connect("127.0.0.1:2022")
                    .await?
                    .into_split();
                (r.compat(), w.compat_write())
            };
            tokio::task::spawn(async move {
                futures::io::copy(yamux_stream_read, &mut connection_write)
                    .await
                    .unwrap();
            });
            tokio::task::spawn(async move {
                futures::io::copy(connection_read, &mut yamux_write)
                    .await
                    .unwrap();
            });
            Ok(())
        })
        .await?;
    Ok(())
}
