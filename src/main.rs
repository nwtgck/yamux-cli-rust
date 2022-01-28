mod stdio;

use anyhow::Result;

#[tokio::main]
async fn main() {
    serve().await.unwrap();
}

async fn serve() -> Result<()> {
    use futures::{AsyncReadExt, TryStreamExt};

    let stdio = stdio::Stdio::new();
    let yamux_config = yamux::Config::default();
    let yamux_server = yamux::Connection::new(stdio, yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_server)
        .try_for_each_concurrent(None, |yamux_stream| async move {
            let (yamux_stream_read, yamux_write) = yamux_stream.split();
            let (mut connection_read, mut connection_write) =
                tokio::net::TcpStream::connect("127.0.0.1:2022")
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
