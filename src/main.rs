mod stdio;

use anyhow::Result;

#[tokio::main]
async fn main() {
    eprintln!("start");
    serve().await.unwrap();
}

async fn serve() -> Result<()> {
    use futures::{AsyncReadExt, TryStreamExt};

    let stdio = stdio::Stdio::new();
    let mut yamux_config = yamux::Config::default();
    // yamux_config.set_max_buffer_size(23);
    // yamux_config.set_read_after_close(true);
    let yamux_server = yamux::Connection::new(stdio, yamux_config, yamux::Mode::Server);
    yamux::into_stream(yamux_server)
        .try_for_each_concurrent(None, |yamux_stream| async move {
            eprintln!("new yamux_stream");
            use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};
            let (yamux_stream_read, yamux_write) = yamux_stream.split();
            let (mut connection_read, mut connection_write) =
                tokio::net::TcpStream::connect("127.0.0.1:2022")
                    .await?
                    .into_split();

            tokio::spawn(async move {
                let mut buf = [0u8; 16];
                copy_with_buffer(
                    &mut yamux_stream_read.compat(),
                    &mut connection_write,
                    &mut buf,
                )
                .await
                .unwrap();
            });
            tokio::spawn(async move {
                let mut buf = [0u8; 16];
                copy_with_buffer(
                    &mut connection_read,
                    &mut yamux_write.compat_write(),
                    &mut buf,
                )
                .await
                .unwrap();
                // tokio::io::copy(&mut connection_read, &mut yamux_write.compat_write())
                //     .await
                //     .unwrap();
            });
            Ok(())
        })
        .await?;
    Ok(())
}

async fn copy_with_buffer<R: tokio::io::AsyncRead + Unpin, W: tokio::io::AsyncWrite + Unpin>(
    reader: &mut R,
    writer: &mut W,
    buf: &mut [u8],
) -> tokio::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    eprintln!("copy with buffer start");
    loop {
        let n = reader.read(buf).await?;
        eprintln!("read {:?} in copy", n);
        if n <= 0 {
            break;
        }
        writer.write_all(&buf[..n]).await?;
        writer.flush().await?;
    }
    Ok(())
}
