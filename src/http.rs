//! Optimized HTTP client configuration for AI providers

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use once_cell::sync::Lazy;

/// Global shared HTTP client for all AI providers
pub static SHARED_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    Arc::new(create_optimized_client(Duration::from_secs(30))
        .expect("Failed to create shared HTTP client"))
});

/// Create an optimized HTTP client with connection pooling and keepalive
pub fn create_optimized_client(timeout: Duration) -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Duration::from_secs(60))
        .http2_adaptive_window(true)
        .use_rustls_tls()
        .user_agent(format!("chatdelta/{}", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Configuration for provider-specific HTTP clients
#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    pub pool_idle_timeout: Duration,
    pub tcp_keepalive: Option<Duration>,
    pub http2_adaptive_window: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
            tcp_keepalive: Some(Duration::from_secs(60)),
            http2_adaptive_window: true,
        }
    }
}

impl HttpConfig {
    /// Create optimized config for OpenAI
    pub fn for_openai() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 20, // Higher for frequent requests
            ..Default::default()
        }
    }
    
    /// Create optimized config for Claude
    pub fn for_claude() -> Self {
        Self {
            request_timeout: Duration::from_secs(45), // Claude can be slower
            connect_timeout: Duration::from_secs(15), // More lenient
            ..Default::default()
        }
    }
    
    /// Create optimized config for Gemini
    pub fn for_gemini() -> Self {
        Self {
            request_timeout: Duration::from_secs(25), // Gemini is typically faster
            ..Default::default()
        }
    }
    
    /// Build a client from this configuration
    pub fn build_client(&self) -> Result<Client, reqwest::Error> {
        let mut builder = Client::builder()
            .timeout(self.request_timeout)
            .connect_timeout(self.connect_timeout)
            .pool_idle_timeout(self.pool_idle_timeout)
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .http2_adaptive_window(self.http2_adaptive_window)
            .use_rustls_tls()
            .user_agent(format!("chatdelta/{}", env!("CARGO_PKG_VERSION")));
            
        if let Some(keepalive) = self.tcp_keepalive {
            builder = builder.tcp_keepalive(keepalive);
        }
        
        builder.build()
    }
}

/// Get or create a provider-specific HTTP client
pub fn get_provider_client(provider: &str) -> Arc<Client> {
    match provider.to_lowercase().as_str() {
        "openai" | "gpt" | "chatgpt" => {
            static OPENAI_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
                Arc::new(HttpConfig::for_openai().build_client()
                    .expect("Failed to create OpenAI HTTP client"))
            });
            OPENAI_CLIENT.clone()
        }
        "claude" | "anthropic" => {
            static CLAUDE_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
                Arc::new(HttpConfig::for_claude().build_client()
                    .expect("Failed to create Claude HTTP client"))
            });
            CLAUDE_CLIENT.clone()
        }
        "gemini" | "google" => {
            static GEMINI_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
                Arc::new(HttpConfig::for_gemini().build_client()
                    .expect("Failed to create Gemini HTTP client"))
            });
            GEMINI_CLIENT.clone()
        }
        _ => SHARED_CLIENT.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_http_config_defaults() {
        let config = HttpConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
    }
    
    #[test]
    fn test_provider_specific_configs() {
        let openai = HttpConfig::for_openai();
        assert_eq!(openai.pool_max_idle_per_host, 20);
        
        let claude = HttpConfig::for_claude();
        assert_eq!(claude.request_timeout, Duration::from_secs(45));
        
        let gemini = HttpConfig::for_gemini();
        assert_eq!(gemini.request_timeout, Duration::from_secs(25));
    }
}