//! Performance metrics collection for ChatDelta clients

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Metrics collector for AI client performance
#[derive(Debug, Clone)]
pub struct ClientMetrics {
    pub requests_total: Arc<AtomicU64>,
    pub requests_successful: Arc<AtomicU64>,
    pub requests_failed: Arc<AtomicU64>,
    pub total_latency_ms: Arc<AtomicU64>,
    pub total_tokens_used: Arc<AtomicU64>,
    pub cache_hits: Arc<AtomicU64>,
    pub cache_misses: Arc<AtomicU64>,
}

impl Default for ClientMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            requests_total: Arc::new(AtomicU64::new(0)),
            requests_successful: Arc::new(AtomicU64::new(0)),
            requests_failed: Arc::new(AtomicU64::new(0)),
            total_latency_ms: Arc::new(AtomicU64::new(0)),
            total_tokens_used: Arc::new(AtomicU64::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Record a request and its outcome
    pub fn record_request(&self, success: bool, latency_ms: u64, tokens: Option<u32>) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        
        if success {
            self.requests_successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);
        }
        
        if let Some(tokens) = tokens {
            self.total_tokens_used.fetch_add(tokens as u64, Ordering::Relaxed);
        }
    }
    
    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get a snapshot of current metrics
    pub fn get_stats(&self) -> MetricsSnapshot {
        let total = self.requests_total.load(Ordering::Relaxed);
        let cache_total = self.cache_hits.load(Ordering::Relaxed) + self.cache_misses.load(Ordering::Relaxed);
        
        MetricsSnapshot {
            requests_total: total,
            requests_successful: self.requests_successful.load(Ordering::Relaxed),
            requests_failed: self.requests_failed.load(Ordering::Relaxed),
            success_rate: if total > 0 {
                (self.requests_successful.load(Ordering::Relaxed) as f64 / total as f64) * 100.0
            } else { 0.0 },
            average_latency_ms: if total > 0 {
                self.total_latency_ms.load(Ordering::Relaxed) / total
            } else { 0 },
            total_tokens_used: self.total_tokens_used.load(Ordering::Relaxed),
            cache_hit_rate: if cache_total > 0 {
                (self.cache_hits.load(Ordering::Relaxed) as f64 / cache_total as f64) * 100.0
            } else { 0.0 },
        }
    }
    
    /// Reset all metrics to zero
    pub fn reset(&self) {
        self.requests_total.store(0, Ordering::Relaxed);
        self.requests_successful.store(0, Ordering::Relaxed);
        self.requests_failed.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
        self.total_tokens_used.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

/// A snapshot of metrics at a point in time
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub success_rate: f64,
    pub average_latency_ms: u64,
    pub total_tokens_used: u64,
    pub cache_hit_rate: f64,
}

impl MetricsSnapshot {
    /// Get a human-readable summary of the metrics
    pub fn summary(&self) -> String {
        format!(
            "Requests: {} (Success: {:.1}%), Avg Latency: {}ms, Tokens: {}, Cache Hit: {:.1}%",
            self.requests_total,
            self.success_rate,
            self.average_latency_ms,
            self.total_tokens_used,
            self.cache_hit_rate
        )
    }
}

/// Request timer for measuring latency
pub struct RequestTimer {
    start: Instant,
    metrics: ClientMetrics,
}

impl RequestTimer {
    /// Start a new request timer
    pub fn new(metrics: ClientMetrics) -> Self {
        Self {
            start: Instant::now(),
            metrics,
        }
    }
    
    /// Complete the request and record metrics
    pub fn complete(self, success: bool, tokens: Option<u32>) {
        let latency_ms = self.start.elapsed().as_millis() as u64;
        self.metrics.record_request(success, latency_ms, tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_recording() {
        let metrics = ClientMetrics::new();
        
        // Record some successful requests
        metrics.record_request(true, 100, Some(50));
        metrics.record_request(true, 200, Some(75));
        
        // Record a failed request
        metrics.record_request(false, 50, None);
        
        let stats = metrics.get_stats();
        assert_eq!(stats.requests_total, 3);
        assert_eq!(stats.requests_successful, 2);
        assert_eq!(stats.requests_failed, 1);
        assert_eq!(stats.average_latency_ms, 116); // (100+200+50)/3
        assert_eq!(stats.total_tokens_used, 125);
        assert!(stats.success_rate > 66.0 && stats.success_rate < 67.0);
    }
    
    #[test]
    fn test_cache_metrics() {
        let metrics = ClientMetrics::new();
        
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        
        let stats = metrics.get_stats();
        assert!(stats.cache_hit_rate > 66.0 && stats.cache_hit_rate < 67.0);
    }
}