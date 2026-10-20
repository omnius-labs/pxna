use chrono::{DateTime, Utc};

use crate::{model::User, prelude::*};

/// クライアントがセッションの token を受け取り、送る方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTransport {
    Cookie,
    Bearer,
}

impl SessionTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cookie => "cookie",
            Self::Bearer => "bearer",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "cookie" => Ok(Self::Cookie),
            "bearer" => Ok(Self::Bearer),
            // 保存された値だけを読むので、未知の値は入力の誤りではなく保存データの破損である。
            other => Err(Error::new(ErrorKind::UnexpectedError).with_message(format!("unknown session transport: {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub transport: SessionTransport,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// setup とログインで作ったセッション。token の平文はこの値にだけ現れ、保存されない。
#[derive(Debug, Clone)]
pub struct IssuedSession {
    pub user: User,
    pub session: Session,
    pub token: String,
}

/// token から照合できた、有効なセッションとそのユーザ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    pub user: User,
    pub session: Session,
}
