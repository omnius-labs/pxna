use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;
use tokio_util::bytes::{Bytes, BytesMut};

#[allow(unused)]
pub struct BytesSink {
    buf: BytesMut,
}

impl BytesSink {
    #[allow(unused)]
    pub fn new() -> Self {
        Self { buf: BytesMut::new() }
    }

    #[allow(unused)]
    pub fn into_bytes(self) -> Bytes {
        self.buf.freeze()
    }
}

impl AsyncWrite for BytesSink {
    fn poll_write(mut self: Pin<&mut Self>, _: &mut Context<'_>, data: &[u8]) -> Poll<std::io::Result<usize>> {
        self.buf.extend_from_slice(data);
        Poll::Ready(Ok(data.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
