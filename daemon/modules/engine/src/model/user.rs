use chrono::{DateTime, Utc};

use crate::prelude::*;

/// ユーザの役割。いまはオーナーだけで、役割が増えても保存形式を変えずに済むよう列挙で持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Owner,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "owner" => Ok(Self::Owner),
            // 保存された値だけを読むので、未知の値は入力の誤りではなく保存データの破損である。
            other => Err(Error::new(ErrorKind::UnexpectedError).with_message(format!("unknown user role: {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
