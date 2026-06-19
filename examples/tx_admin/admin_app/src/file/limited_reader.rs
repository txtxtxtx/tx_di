//! 带大小限制的流式读取器
//!
//! 包装任意 `AsyncRead`，边读边计数。超过 `max_bytes` 时返回 IO 错误，
//! 上层（`FileAppService`）将其转换为 `FileStorageErr::FileTooLarge`。
//!
//! # 用途
//! - multipart 文件上传时，HTTP body 没有可靠的 `Content-Length`
//! - 此包装器在数据流经时透明拦截，超限即断，避免写存储后再回滚

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

/// 大小感知 `AsyncRead` 包装器
pub struct LimitedAsyncRead<R> {
    inner: R,
    max_bytes: u64,
    bytes_read: u64,
}

impl<R> LimitedAsyncRead<R> {
    /// 创建带大小限制的读取器
    pub fn new(inner: R, max_bytes: u64) -> Self {
        Self {
            inner,
            max_bytes,
            bytes_read: 0,
        }
    }

    /// 实际已读取的字节数（写入成功后用于确定文件大小）
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for LimitedAsyncRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();

        // 已超限，直接报错
        let remaining = this.max_bytes.saturating_sub(this.bytes_read);
        if remaining == 0 {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("文件大小超限: 已读取 {} 字节，上限 {} 字节", this.bytes_read, this.max_bytes),
            )));
        }

        // 限制本次可读字节数 = min(剩余限额, buf 可用空间)
        let cap = (remaining as usize).min(buf.remaining());
        let mut sub = buf.take(cap);

        let filled_before = sub.filled().len();
        let poll = Pin::new(&mut this.inner).poll_read(cx, &mut sub);
        let filled_after = sub.filled().len();
        let delta = filled_after - filled_before;
        this.bytes_read += delta as u64;

        // `ReadBuf::take()` 不会自动将 filled 传回父 buffer——必须手动完成
        // (子 ReadBuf 独立管理 filled/initialized，数据在底层字节数组，但父 buffer 不知道)
        if delta > 0 {
            // SAFETY: 底层字节已在子 ReadBuf 中初始化（由内层 reader 写入）
            unsafe {
                buf.assume_init(delta);
            }
            buf.advance(delta);
        }

        poll
    }
}
