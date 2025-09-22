//! Observability pipeline for metrics export and structured logging

use crate::ClientMetrics;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[cfg(feature = "metrics-export")]
use prometheus::{Encoder, TextEncoder, Registry, Counter, Histogram, HistogramOpts, CounterOpts};

#[cfg(feature = "metrics-export")]
use opentelemetry::{
    metrics::{Meter, Counter as OtelCounter, Histogram as OtelHistogram},
    KeyValue,
};

/// Initialize tracing with structured logging
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .json();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    info!("ChatDelta observability initialized");
}

/// Metrics exporter trait
pub trait MetricsExporter: Send + Sync {
    /// Export current metrics
    fn export(&self, metrics: &ClientMetrics) -> String;

    /// Get exporter name
    fn name(&self) -> &str;
}

#[cfg(feature = "metrics-export")]
pub struct PrometheusExporter {
    registry: Registry,
    requests_total: Counter,
    requests_successful: Counter,
    requests_failed: Counter,
    request_duration: Histogram,
    tokens_used: Counter,
    cache_hits: Counter,
    cache_misses: Counter,
}

#[cfg(feature = "metrics-export")]
impl PrometheusExporter {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Registry::new();

        let requests_total = Counter::with_opts(
            CounterOpts::new("chatdelta_requests_total", "Total number of API requests")
        )?;

        let requests_successful = Counter::with_opts(
            CounterOpts::new("chatdelta_requests_successful", "Number of successful API requests")
        )?;

        let requests_failed = Counter::with_opts(
            CounterOpts::new("chatdelta_requests_failed", "Number of failed API requests")
        )?;

        let request_duration = Histogram::with_opts(
            HistogramOpts::new("chatdelta_request_duration_ms", "Request duration in milliseconds")
                .buckets(vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0])
        )?;

        let tokens_used = Counter::with_opts(
            CounterOpts::new("chatdelta_tokens_used_total", "Total tokens consumed")
        )?;

        let cache_hits = Counter::with_opts(
            CounterOpts::new("chatdelta_cache_hits_total", "Total cache hits")
        )?;

        let cache_misses = Counter::with_opts(
            CounterOpts::new("chatdelta_cache_misses_total", "Total cache misses")
        )?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(requests_successful.clone()))?;
        registry.register(Box::new(requests_failed.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(tokens_used.clone()))?;
        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;

        Ok(Self {
            registry,
            requests_total,
            requests_successful,
            requests_failed,
            request_duration,
            tokens_used,
            cache_hits,
            cache_misses,
        })
    }

    /// Update Prometheus metrics from ClientMetrics
    pub fn update(&self, metrics: &ClientMetrics) {
        use std::sync::atomic::Ordering;

        let snapshot = metrics.snapshot();

        // Set counters to current values
        self.requests_total.inc_by(
            snapshot.requests_total as f64 - self.requests_total.get()
        );
        self.requests_successful.inc_by(
            snapshot.requests_successful as f64 - self.requests_successful.get()
        );
        self.requests_failed.inc_by(
            snapshot.requests_failed as f64 - self.requests_failed.get()
        );
        self.tokens_used.inc_by(
            snapshot.total_tokens_used as f64 - self.tokens_used.get()
        );
        self.cache_hits.inc_by(
            snapshot.cache_hits as f64 - self.cache_hits.get()
        );
        self.cache_misses.inc_by(
            snapshot.cache_misses as f64 - self.cache_misses.get()
        );

        // Update histogram with average latency if we have requests
        if snapshot.requests_total > 0 {
            let avg_latency = snapshot.average_latency_ms.unwrap_or(0.0);
            self.request_duration.observe(avg_latency);
        }
    }
}

#[cfg(feature = "metrics-export")]
impl MetricsExporter for PrometheusExporter {
    fn export(&self, metrics: &ClientMetrics) -> String {
        self.update(metrics);

        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    fn name(&self) -> &str {
        "prometheus"
    }
}

/// Simple text exporter for metrics (always available)
pub struct TextExporter;

impl MetricsExporter for TextExporter {
    fn export(&self, metrics: &ClientMetrics) -> String {
        let snapshot = metrics.snapshot();
        format!(
            "ChatDelta Metrics:\n\
            Requests Total: {}\n\
            Requests Successful: {}\n\
            Requests Failed: {}\n\
            Average Latency: {:.2}ms\n\
            Total Tokens Used: {}\n\
            Cache Hits: {}\n\
            Cache Misses: {}\n\
            Success Rate: {:.2}%",
            snapshot.requests_total,
            snapshot.requests_successful,
            snapshot.requests_failed,
            snapshot.average_latency_ms.unwrap_or(0.0),
            snapshot.total_tokens_used,
            snapshot.cache_hits,
            snapshot.cache_misses,
            snapshot.success_rate * 100.0
        )
    }

    fn name(&self) -> &str {
        "text"
    }
}

/// Observability context for request tracing
pub struct ObservabilityContext {
    pub request_id: String,
    pub provider: String,
    pub model: String,
    pub metrics: Arc<ClientMetrics>,
}

impl ObservabilityContext {
    pub fn new(provider: String, model: String, metrics: Arc<ClientMetrics>) -> Self {
        use rand::Rng;
        let request_id = format!("{:016x}", rand::thread_rng().gen::<u64>());

        Self {
            request_id,
            provider,
            model,
            metrics,
        }
    }

    /// Create a tracing span for this request
    pub fn span(&self) -> tracing::Span {
        tracing::span!(
            Level::INFO,
            "ai_request",
            request_id = %self.request_id,
            provider = %self.provider,
            model = %self.model
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_exporter() {
        let metrics = ClientMetrics::new();
        metrics.record_request(true, 100, Some(50));
        metrics.record_request(false, 200, None);
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        let exporter = TextExporter;
        let output = exporter.export(&metrics);

        assert!(output.contains("Requests Total: 2"));
        assert!(output.contains("Requests Successful: 1"));
        assert!(output.contains("Requests Failed: 1"));
        assert!(output.contains("Cache Hits: 1"));
        assert!(output.contains("Cache Misses: 1"));
    }
}