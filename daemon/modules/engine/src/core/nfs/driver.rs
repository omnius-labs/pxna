use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt as _};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};
use tokio_util::bytes::{Bytes, BytesMut};

use crate::{
    core::nfs::{
        materialized::MaterializedFile,
        materialized::OpenedFile,
        staging::{ConflictPolicy, Publish, StagingFile},
    },
    prelude::*,
};

pub mod archive;
pub mod local;

pub type FsEntrySummaryStream = Pin<Box<dyn Stream<Item = Result<FsEntrySummary>> + Send + 'static>>;

pub type FsEntryStream = Pin<Box<dyn Stream<Item = Result<FsEntry>> + Send + 'static>>;

/// `read_file` の 1 call 上限。protocol limit であり設定化しない。
///
/// これを超える読み取りは `open_file` の stream で行う。
pub const MAX_READ_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// 1 つの filesystem namespace に bind された操作 contract。
///
/// path はすべて bind 済み namespace 内の `&str` であり、driver は `NestedPath` を知らない。
/// 入れ子の解決は `StorageService` の責務である。
#[async_trait]
pub trait FileStorageDriver: Send + Sync {
    async fn get_entry(&self, path: &str) -> Result<FsEntry>;

    /// `recursive` が `false` なら直下だけ、`true` なら全 descendant を返す。
    /// 順序は contract に含めない。
    async fn find_entries(&self, path: &str, recursive: bool) -> Result<FsEntryStream>;

    /// 一覧の leaf と同じく symlink を追わない詳細。archive は索引の Entry をそのまま返す。
    async fn get_listing_entry(&self, path: &str) -> Result<FsEntry> {
        self.get_entry(path).await
    }

    /// 直下の名前と種別。属性が既にある namespace は既存の一覧から写せる。
    async fn find_entry_summaries(&self, path: &str) -> Result<FsEntrySummaryStream> {
        let entries = self.find_entries(path, false).await?;
        Ok(Box::pin(entries.map(|entry| entry.map(FsEntrySummary::from_entry))))
    }

    async fn exists_file(&self, path: &str) -> Result<bool>;
    async fn exists_dir(&self, path: &str) -> Result<bool>;

    /// 対象 entry を物理 file として取り出す。driver が実装する中心操作である。
    async fn materialize_file(&self, path: &str) -> Result<MaterializedFile>;

    async fn open_file(&self, path: &str) -> Result<OpenedFile> {
        self.materialize_file(path).await?.open().await
    }

    /// engine 内部の小さな読み取りのためにある。
    ///
    /// HTTP の content 応答はこれを経由せず、`open_file` を stream として消費する。
    async fn read_file(&self, path: &str, offset: u64, limit: u64) -> Result<Bytes> {
        if limit > MAX_READ_FILE_BYTES {
            return Err(Error::new(ErrorKind::LimitExceeded).with_message(format!("read_file limit {limit} exceeds {MAX_READ_FILE_BYTES}")));
        }
        if limit == 0 {
            return Ok(Bytes::new());
        }

        let mut opened = self.open_file(path).await?;
        let len = opened.len().await?;
        if offset >= len {
            return Ok(Bytes::new());
        }

        // 要求 limit ではなく、実際に読める量だけを確保する。
        let to_read = limit.min(len - offset);
        opened.seek(std::io::SeekFrom::Start(offset)).await?;

        let mut buf = BytesMut::with_capacity(to_read as usize);
        while (buf.len() as u64) < to_read {
            let read = opened.read_buf(&mut buf).await?;
            if read == 0 {
                break;
            }
        }
        buf.truncate(to_read as usize);

        Ok(buf.freeze())
    }

    /// destination と同じ directory に未公開の書き込み先を作る。
    ///
    /// upload session が「staging を作る」「追記する」「publish する」の 3 段をリクエストをまたいで
    /// 行うため、これを primitive にしている。
    async fn create_staging(&self, dst_dir: &str) -> Result<StagingFile>;

    /// 1 段のみ。`create_parents` は持たない。
    async fn make_dir(&self, path: &str) -> Result<()>;

    async fn remove_file(&self, path: &str) -> Result<()>;

    /// 空のみ。再帰削除は持たない。
    async fn remove_dir(&self, path: &str) -> Result<()>;

    /// 同一 driver 内のみ。別 filesystem をまたぐ場合の退避は `StorageService` が判断する。
    async fn rename(&self, src: &str, dst: &str, conflict: ConflictPolicy) -> Result<Publish>;
}
