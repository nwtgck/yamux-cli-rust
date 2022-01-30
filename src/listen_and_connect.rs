pub enum Listener {
    TcpListener(tokio::net::TcpListener),
    #[cfg(unix)]
    UnixListener(tokio::net::UnixListener),
}

#[cfg(unix)]
#[auto_enums::enum_derive(tokio1::AsyncRead)]
pub enum TcpOrUnixAsyncRead {
    TcpAsyncRead(tokio::net::tcp::OwnedReadHalf),
    UnixAsyncRead(tokio::net::unix::OwnedReadHalf),
}

#[cfg(not(unix))]
#[auto_enums::enum_derive(tokio1::AsyncRead)]
pub enum TcpOrUnixAsyncRead {
    TcpAsyncRead(tokio::net::tcp::OwnedReadHalf),
}

#[cfg(unix)]
#[auto_enums::enum_derive(tokio1::AsyncWrite)]
pub enum TcpOrUnixAsyncWrite {
    TcpAsyncWrite(tokio::net::tcp::OwnedWriteHalf),
    UnixAsyncWrite(tokio::net::unix::OwnedWriteHalf),
}

#[cfg(not(unix))]
#[auto_enums::enum_derive(tokio1::AsyncWrite)]
pub enum TcpOrUnixAsyncWrite {
    TcpAsyncWrite(tokio::net::tcp::OwnedWriteHalf),
}

fn tcp_stream_to_enum(
    tcp_stream: tokio::net::TcpStream,
) -> (TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite) {
    let (r, w) = tcp_stream.into_split();
    (
        TcpOrUnixAsyncRead::TcpAsyncRead(r),
        TcpOrUnixAsyncWrite::TcpAsyncWrite(w),
    )
}

#[cfg(unix)]
fn unix_stream_to_enum(
    unix_stream: tokio::net::UnixStream,
) -> (TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite) {
    let (r, w) = unix_stream.into_split();
    (
        TcpOrUnixAsyncRead::UnixAsyncRead(r),
        TcpOrUnixAsyncWrite::UnixAsyncWrite(w),
    )
}

impl Listener {
    pub fn poll_accept(
        &self,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<tokio::io::Result<(TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite)>> {
        match self {
            Listener::TcpListener(tcp_listener) => {
                let poll = tcp_listener.poll_accept(cx);
                poll.map_ok(|(stream, _)| tcp_stream_to_enum(stream))
            }
            #[cfg(unix)]
            Listener::UnixListener(unix_listener) => {
                let poll = unix_listener.poll_accept(cx);
                poll.map_ok(|(stream, _)| unix_stream_to_enum(stream))
            }
        }
    }
}

#[derive(Clone)]
pub enum ConnectSetting {
    TcpConnectSetting {
        host: String,
        port: u16,
    },
    #[cfg(unix)]
    UnixConnectSetting {
        path: String,
    },
}

impl ConnectSetting {
    pub async fn connect(&self) -> tokio::io::Result<(TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite)> {
        match self {
            ConnectSetting::TcpConnectSetting { host, port } => {
                tokio::net::TcpStream::connect((host.as_str(), *port))
                    .await
                    .map(tcp_stream_to_enum)
            }
            #[cfg(unix)]
            ConnectSetting::UnixConnectSetting { path } => tokio::net::UnixStream::connect(path)
                .await
                .map(unix_stream_to_enum),
        }
    }
}
