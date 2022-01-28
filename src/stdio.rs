use pin_project_lite::pin_project;
use std::io::Write;
use std::pin::Pin;
use std::task::{Context, Poll};

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
        return self.project().stdin.poll_read(cx, buf);
    }
}

impl futures::AsyncWrite for Stdio {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let poll = self.project().stdout.poll_write(cx, buf);
        // FIXME: dirty but works (should not flush in poll_write, should flush in copy or something)
        // NOTE: tokio::spawn() with tokio::io::stdout().flush().await and tokio::task::spawn_blocking do not work
        std::thread::spawn(|| {
            std::io::stdout().flush().unwrap();
        });
        return poll;
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return self.project().stdout.poll_flush(cx);
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return self.project().stdout.poll_close(cx);
    }
}
