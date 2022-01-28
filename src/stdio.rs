use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

pin_project! {
    pub struct Stdio {
        #[pin]
        stdin: tokio_util::compat::Compat<tokio::io::Stdin>,
        #[pin]
        stdout: tokio_util::compat::Compat<tokio::io::Stdout>,
    }
}

impl Stdio {
    pub fn new() -> Self {
        use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
        Stdio {
            stdin: tokio::io::stdin().compat(),
            stdout: tokio::io::stdout().compat_write(),
        }
    }
}

impl futures::AsyncRead for Stdio {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        use tokio_util::compat::TokioAsyncReadCompatExt;
        let mut this = self.project();
        eprintln!("> poll read: {:?}", buf);
        // let res = Pin::new(&mut tokio::io::stdin().compat()).poll_read(cx, buf);
        let res = this.stdin.poll_read(cx, buf);
        eprintln!("< poll read: {:?}", buf);
        return res;
        // eprintln!("> poll read: {:?}", buf);
        // let res = Pin::new(&mut futures::io::AllowStdIo::new(std::io::stdin())).poll_read(cx, buf);
        // eprintln!("< poll read: {:?}", buf);
        // return res;
    }
}

impl futures::AsyncWrite for Stdio {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        use tokio_util::compat::TokioAsyncWriteCompatExt;
        let mut this = self.project();

        eprintln!("> poll write: {:?}", buf);
        // let res =
        //     Pin::new(&mut futures::io::AllowStdIo::new(std::io::stdout())).poll_write(cx, buf);
        // std::io::stdout().flush();
        // let res = Pin::new(&mut tokio::io::stdout().compat_write()).poll_write(cx, buf);
        let res = this.stdout.poll_write(cx, buf);
        eprintln!("< poll write: {:?}", buf);
        return res;
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        use tokio_util::compat::TokioAsyncWriteCompatExt;
        let mut this = self.project();
        // return Pin::new(&mut tokio::io::stdout().compat_write()).poll_flush(cx);
        return this.stdout.poll_flush(cx);
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        use tokio_util::compat::TokioAsyncWriteCompatExt;
        let mut this = self.project();
        // return Pin::new(&mut tokio::io::stdout().compat_write()).poll_flush(cx);
        return this.stdout.poll_close(cx);
    }
}
