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
        flush_tx: tokio::sync::mpsc::Sender<()>,
    }
}

impl Stdio {
    pub fn new() -> Self {
        use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

        // FIXME: dirty but works (should not flush in poll_write, should flush in copy or something)
        // Maybe fixed by https://github.com/tokio-rs/tokio/pull/4348
        let (flush_tx, mut flush_rx) = tokio::sync::mpsc::channel::<()>(1);
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        std::thread::spawn(move || {
            while let Some(_) = tokio_runtime.block_on(flush_rx.recv()) {
                std::io::stdout().flush().unwrap();
            }
        });
        Stdio {
            stdin: tokio::io::stdin().compat(),
            stdout: tokio::io::stdout().compat_write(),
            flush_tx,
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
        let this = self.project();
        let poll = this.stdout.poll_write(cx, buf);
        if poll.is_ready() {
            let flush_tx = this.flush_tx.clone();
            tokio::task::spawn(async move {
                flush_tx.send(()).await.unwrap();
            });
        }
        return poll;
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return self.project().stdout.poll_flush(cx);
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return self.project().stdout.poll_close(cx);
    }
}
