use base64::Engine as _;
use ring::rand::{SecureRandom as _, SystemRandom};
use sha3::{Digest as _, Sha3_256};

use crate::prelude::*;

const TOKEN_BYTE_COUNT: usize = 32;

/// セッションの token。平文はクライアントへ渡すときにだけ使い、保存と照合には SHA3-256 を使う。
pub struct SessionToken;

impl SessionToken {
    pub fn generate() -> Result<String> {
        let mut bytes = [0_u8; TOKEN_BYTE_COUNT];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| Error::new(ErrorKind::CryptoError).with_message("failed to generate a session token"))?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn hash(token: &str) -> Vec<u8> {
        Sha3_256::digest(token.as_bytes()).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unique_url_safe_tokens_test() -> testresult::TestResult {
        let a = SessionToken::generate()?;
        let b = SessionToken::generate()?;
        assert_ne!(a, b);
        assert_eq!(a.len(), 43, "32 byte を padding なしの base64url で表す");
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')));
        assert_eq!(SessionToken::hash(&a).len(), 32);
        Ok(())
    }
}
