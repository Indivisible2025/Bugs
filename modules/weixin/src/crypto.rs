//! 微信消息签名验证（SHA1）

use sha1::{Digest, Sha1};

/// 验证微信回调签名
pub fn verify_signature(token: &str, timestamp: &str, nonce: &str, signature: &str) -> bool {
    let mut items = [token, timestamp, nonce];
    items.sort();
    let sorted = items.concat();
    let mut hasher = Sha1::new();
    hasher.update(sorted.as_bytes());
    hex::encode(hasher.finalize()) == signature
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verify_signature_works() {
        let mut items = vec!["test", "1409304348", "xxxxxx"];
        items.sort();
        let sorted = items.concat();
        let mut h = Sha1::new();
        h.update(sorted.as_bytes());
        let sig = hex::encode(h.finalize());
        assert!(verify_signature("test", "1409304348", "xxxxxx", &sig));
    }
}
