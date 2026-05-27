#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_config() {
        let cfg = bugs_core::types::BugsConfig::default();
        assert_eq!(cfg.network.local.port, 8742);
    }
}
