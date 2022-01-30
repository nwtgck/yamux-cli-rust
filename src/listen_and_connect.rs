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
    pub async fn accept(&self) -> tokio::io::Result<(TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite)> {
        match self {
            Listener::TcpListener(tcp_listener) => tcp_listener
                .accept()
                .await
                .map(|(stream, _)| tcp_stream_to_enum(stream)),
            #[cfg(unix)]
            Listener::UnixListener(unix_listener) => unix_listener
                .accept()
                .await
                .map(|(stream, _)| unix_stream_to_enum(stream)),
        }
    }
}

#[derive(Clone)]
pub enum Connector {
    Tcp {
        host: String,
        port: u16,
    },
    #[cfg(unix)]
    Unix {
        path: String,
    },
}

impl Connector {
    pub async fn connect(&self) -> tokio::io::Result<(TcpOrUnixAsyncRead, TcpOrUnixAsyncWrite)> {
        match self {
            Connector::Tcp { host, port } => tokio::net::TcpStream::connect((host.as_str(), *port))
                .await
                .map(tcp_stream_to_enum),
            #[cfg(unix)]
            Connector::Unix { path } => tokio::net::UnixStream::connect(path)
                .await
                .map(unix_stream_to_enum),
        }
    }
}
