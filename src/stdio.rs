use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Stdio;

impl futures::AsyncRead for Stdio {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        eprintln!("> poll read: {:?}", buf);
        let res = Pin::new(&mut futures::io::AllowStdIo::new(std::io::stdin())).poll_read(cx, buf);
        eprintln!("< poll read: {:?}", buf);
        return res;
    }
}

impl futures::AsyncWrite for Stdio {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        eprintln!("> poll write: {:?}", buf);
        let res =
            Pin::new(&mut futures::io::AllowStdIo::new(std::io::stdout())).poll_write(cx, buf);
        eprintln!("< poll write: {:?}", buf);
        return res;
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return Pin::new(&mut futures::io::AllowStdIo::new(std::io::stdout())).poll_flush(cx);
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return Pin::new(&mut futures::io::AllowStdIo::new(std::io::stdout())).poll_close(cx);
    }
}
