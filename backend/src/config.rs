#[derive(Debug, Clone)]
pub struct AppConfig {
    pub site_title: String,
    pub pin: Option<String>,
    pub cookie_max_age_hours: i64,
    pub page_history_cookie_age_days: i64,
    pub max_attempts: usize,
    pub lockout_time_minutes: u64,
    pub trust_proxy: bool,
    pub trusted_proxies: Vec<ipnet::IpNet>,
    pub base_url: String,
    pub node_env: String,
    pub version: String,
    pub allowed_origins: String,
}

impl AppConfig {
    pub fn load_from_env(port: u16) -> Self {
        let site_title = std::env::var("RUSTPAD_TITLE")
            .or_else(|_| std::env::var("RUSTPAD_SITE_TITLE"))
            .or_else(|_| std::env::var("SITE_TITLE"))
            .unwrap_or_else(|_| "RustPad".to_string());

        let pin = std::env::var("RUSTPAD_PIN")
            .or_else(|_| std::env::var("PIN"))
            .ok()
            .filter(|s| !s.is_empty())
            .filter(|p| p.len() >= 4 && p.len() <= 10 && p.chars().all(|c| c.is_ascii_digit()));

        let cookie_max_age_hours = std::env::var("COOKIE_MAX_AGE")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(24);

        let page_history_cookie_age_days = std::env::var("PAGE_HISTORY_COOKIE_AGE")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(365);

        let max_attempts = std::env::var("MAX_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(5);

        let lockout_time_minutes = std::env::var("LOCKOUT_TIME")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(15);

        let trust_proxy = std::env::var("TRUST_PROXY")
            .map(|s| s == "true")
            .unwrap_or(false);

        let trusted_proxy_ips_raw = std::env::var("TRUSTED_PROXY_IPS").unwrap_or_default();
        let mut trusted_proxies = crate::utils::parse_trusted_proxies(&trusted_proxy_ips_raw);

        if trust_proxy && trusted_proxy_ips_raw.trim().is_empty() {
            eprintln!("CRITICAL WARNING: TRUST_PROXY=true but TRUSTED_PROXY_IPS is not set or empty.");
            eprintln!("Trust proxy is disabled for security. Set TRUSTED_PROXY_IPS to enable proxy trust.");
            trusted_proxies.clear();
        }

        let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));
        let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());
        let version = env!("CARGO_PKG_VERSION").to_string();
        let allowed_origins = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());

        Self {
            site_title,
            pin,
            cookie_max_age_hours,
            page_history_cookie_age_days,
            max_attempts,
            lockout_time_minutes,
            trust_proxy: trust_proxy && !trusted_proxy_ips_raw.trim().is_empty(),
            trusted_proxies,
            base_url,
            node_env,
            version,
            allowed_origins,
        }
    }
}
