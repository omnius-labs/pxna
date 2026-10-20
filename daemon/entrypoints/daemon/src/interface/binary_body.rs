use axum::{
    body::{Bytes, to_bytes},
    extract::Request,
};

use crate::prelude::*;

/// 書き込み API の本文を上限付きで読む。
pub struct BinaryBody;

impl BinaryBody {
    pub const MAX_BYTES: usize = 32 * 1024 * 1024;

    /// 上限を超えた本文は、呼び出し側が選んだ `kind` と `message` の error にする。
    pub async fn read(request: Request, kind: ErrorKind, message: &'static str) -> Result<Bytes> {
        to_bytes(request.into_body(), Self::MAX_BYTES).await.map_err(|_| Error::new(kind).with_message(message))
    }
}
