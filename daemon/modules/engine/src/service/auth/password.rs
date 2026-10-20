use argon2::{Argon2, PasswordHasher as _, PasswordVerifier as _};

use crate::prelude::*;

/// argon2id によるパスワードのハッシュ。保存形式は salt と parameter を含む PHC 文字列である。
pub struct PasswordDigest;

impl PasswordDigest {
    /// argon2id は意図的に遅いため、blocking worker 上で計算する。
    pub async fn hash(password: &str) -> Result<String> {
        let password = password.to_string();
        tokio::task::spawn_blocking(move || {
            Argon2::default()
                .hash_password(password.as_bytes())
                .map(|hash| hash.to_string())
                .map_err(|error| Error::new(ErrorKind::CryptoError).with_message(format!("failed to hash a password: {error}")))
        })
        .await
        .map_err(|error| Error::from_error(error, ErrorKind::TaskError))?
    }

    /// 保存されたハッシュが壊れていても、照合の失敗として扱う。
    pub async fn verify(password: &str, hash: &str) -> Result<bool> {
        let password = password.to_string();
        let hash = hash.to_string();
        tokio::task::spawn_blocking(move || Argon2::default().verify_password(password.as_bytes(), hash.as_str()).is_ok())
            .await
            .map_err(|error| Error::from_error(error, ErrorKind::TaskError))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verifies_only_the_original_password_test() -> testresult::TestResult {
        let hash = PasswordDigest::hash("correct horse").await?;
        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains("correct horse"));
        assert!(PasswordDigest::verify("correct horse", &hash).await?);
        assert!(!PasswordDigest::verify("wrong horse", &hash).await?);
        assert!(!PasswordDigest::verify("correct horse", "not a hash").await?);
        Ok(())
    }
}
