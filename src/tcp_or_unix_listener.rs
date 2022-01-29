pub enum Listener {
    TcpListener(tokio::net::TcpListener),
    UnixListener(tokio::net::UnixListener),
}

#[auto_enums::enum_derive(tokio1::AsyncRead)]
pub enum TcpOrUnixAsyncRead {
    TcpAsyncRead(tokio::net::tcp::OwnedReadHalf),
    UnixAsyncRead(tokio::net::unix::OwnedReadHalf),
}

#[auto_enums::enum_derive(tokio1::AsyncWrite)]
pub enum TcpOrUnixAsyncWrite {
    TcpAsyncWrite(tokio::net::tcp::OwnedWriteHalf),
    UnixAsyncWrite(tokio::net::unix::OwnedWriteHalf),
}

impl Listener {
    pub fn poll_accept(
        &self,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<tokio::io::Result<(TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite)>> {
        match self {
            Listener::TcpListener(tcp_listener) => {
                let poll = tcp_listener.poll_accept(cx);
                poll.map_ok(|(tcp_stream, _)| {
                    let (r, w) = tcp_stream.into_split();
                    (
                        TcpOrUnixAsyncRead::TcpAsyncRead(r),
                        TcpOrUnixAsyncWrite::TcpAsyncWrite(w),
                    )
                })
            }
            Listener::UnixListener(unix_listener) => {
                let poll = unix_listener.poll_accept(cx);
                poll.map_ok(|(tcp_stream, _)| {
                    let (r, w) = tcp_stream.into_split();
                    (
                        TcpOrUnixAsyncRead::UnixAsyncRead(r),
                        TcpOrUnixAsyncWrite::UnixAsyncWrite(w),
                    )
                })
            }
        }
    }
}
