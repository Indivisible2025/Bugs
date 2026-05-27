
/// 安全模块——认证、授权、Token 管理
pub struct Security {
    token: String,
}

impl Security {
    pub fn new() -> Self {
        Self {
            token: generate_token(),
        }
    }

    /// 验证 Token（远程连接时）
    pub fn verify(&self, token: &str) -> bool {
        !token.is_empty() && token == self.token
    }

    /// 获取当前 Token
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 重新生成 Token
    pub fn regenerate(&mut self) {
        self.token = generate_token();
    }
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    hex::encode(bytes)
}

impl Default for Security {
    fn default() -> Self {
        Self::new()
    }
}
