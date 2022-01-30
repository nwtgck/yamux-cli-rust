pub enum Listener {
    Tcp(tokio::net::TcpListener),
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
}

#[cfg(unix)]
#[auto_enums::enum_derive(tokio1::AsyncRead)]
pub enum TcpOrUnixOwnedHalfRead {
    Tcp(tokio::net::tcp::OwnedReadHalf),
    Unix(tokio::net::unix::OwnedReadHalf),
}

#[cfg(not(unix))]
#[auto_enums::enum_derive(tokio1::AsyncRead)]
pub enum TcpOrUnixOwnedHalfRead {
    Tcp(tokio::net::tcp::OwnedReadHalf),
}

#[cfg(unix)]
#[auto_enums::enum_derive(tokio1::AsyncWrite)]
pub enum TcpOrUnixOwnedHalfWrite {
    Tcp(tokio::net::tcp::OwnedWriteHalf),
    Unix(tokio::net::unix::OwnedWriteHalf),
}

#[cfg(not(unix))]
#[auto_enums::enum_derive(tokio1::AsyncWrite)]
pub enum TcpOrUnixOwnedHalfWrite {
    Tcp(tokio::net::tcp::OwnedWriteHalf),
}

fn tcp_stream_to_enum(
    tcp_stream: tokio::net::TcpStream,
) -> (TcpOrUnixOwnedHalfRead, TcpOrUnixOwnedHalfWrite) {
    let (r, w) = tcp_stream.into_split();
    (
        TcpOrUnixOwnedHalfRead::Tcp(r),
        TcpOrUnixOwnedHalfWrite::Tcp(w),
    )
}

#[cfg(unix)]
fn unix_stream_to_enum(
    unix_stream: tokio::net::UnixStream,
) -> (TcpOrUnixOwnedHalfRead, TcpOrUnixOwnedHalfWrite) {
    let (r, w) = unix_stream.into_split();
    (
        TcpOrUnixOwnedHalfRead::Unix(r),
        TcpOrUnixOwnedHalfWrite::Unix(w),
    )
}

impl Listener {
    pub async fn accept(
        &self,
    ) -> tokio::io::Result<(TcpOrUnixOwnedHalfRead, TcpOrUnixOwnedHalfWrite)> {
        match self {
            Listener::Tcp(tcp_listener) => tcp_listener
                .accept()
                .await
                .map(|(stream, _)| tcp_stream_to_enum(stream)),
            #[cfg(unix)]
            Listener::Unix(unix_listener) => unix_listener
                .accept()
                .await
                .map(|(stream, _)| unix_stream_to_enum(stream)),
        }
    }
}

#[derive(Clone)]
pub enum Connector<'a> {
    Tcp {
        host: &'a str,
        port: u16,
    },
    #[cfg(unix)]
    Unix {
        path: &'a str,
    },
}

impl<'a> Connector<'a> {
    pub async fn connect(
        self,
    ) -> tokio::io::Result<(TcpOrUnixOwnedHalfRead, TcpOrUnixOwnedHalfWrite)> {
        match self {
            Connector::Tcp { host, port } => tokio::net::TcpStream::connect((host, port))
                .await
                .map(tcp_stream_to_enum),
            #[cfg(unix)]
            Connector::Unix { path } => tokio::net::UnixStream::connect(path)
                .await
                .map(unix_stream_to_enum),
        }
    }
}
