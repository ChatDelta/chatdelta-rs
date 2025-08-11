# ChatDelta v0.5.0 Migration Guide

This guide helps you adopt the new performance and observability features introduced in ChatDelta v0.5.0.

## New Features Overview

### 1. Performance Metrics Collection

ChatDelta now includes built-in metrics collection for monitoring API performance:

```rust
use chatdelta::{ClientMetrics, RequestTimer};

// Create metrics collector
let metrics = ClientMetrics::new();

// Use timer for automatic latency tracking
let timer = RequestTimer::new(metrics.clone());

// Make your API call
let result = client.send_prompt("Hello").await;

// Record the result (automatically captures latency)
timer.complete(result.is_ok(), Some(token_count));

// Get metrics snapshot
let stats = metrics.get_stats();
println!("Success rate: {:.1}%", stats.success_rate);
println!("Average latency: {}ms", stats.average_latency_ms);
```

### 2. Optimized HTTP Clients

Use provider-specific HTTP clients for better performance:

```rust
use chatdelta::http::{get_provider_client, HttpConfig};

// Get optimized client for a specific provider
let http_client = get_provider_client("openai");

// Or create custom configuration
let config = HttpConfig::for_claude(); // Pre-configured for Claude's needs
let custom_client = config.build_client()?;
```

### 3. Enhanced Error Messages

Errors now provide actionable troubleshooting information:

```rust
match client.send_prompt("test").await {
    Ok(response) => println!("{}", response),
    Err(e) => {
        // Errors now include context and solutions
        eprintln!("Error: {}", e);
        // Example output:
        // "Request timed out after attempting to reach api.openai.com. 
        //  Consider increasing timeout or checking network connectivity."
    }
}
```

## Migration Steps

### Step 1: Update Dependency

```toml
[dependencies]
chatdelta = "0.5.0"
```

### Step 2: Add Metrics to Your Application

Replace custom metrics with ChatDelta's built-in system:

**Before (custom metrics):**
```rust
struct MyMetrics {
    requests: u64,
    errors: u64,
}
```

**After (using ClientMetrics):**
```rust
use chatdelta::ClientMetrics;

let metrics = ClientMetrics::new();

// Track all requests automatically
metrics.record_request(success, latency_ms, token_count);

// Get comprehensive statistics
let stats = metrics.get_stats();
```

### Step 3: Use Connection Pooling

Enable connection reuse for better performance:

```rust
use chatdelta::http::SHARED_CLIENT;

// All clients now share optimized HTTP connections
let config = ClientConfig::builder()
    .timeout(Duration::from_secs(30))
    .build();

// Connection pooling is automatic
let client = create_client("openai", api_key, "gpt-4", config)?;
```

### Step 4: Display Metrics

Show performance metrics to users:

```rust
use chatdelta::{ClientMetrics, MetricsSnapshot};

fn display_metrics(snapshot: MetricsSnapshot) {
    println!("Performance Report:");
    println!("  Total Requests: {}", snapshot.requests_total);
    println!("  Success Rate: {:.1}%", snapshot.success_rate);
    println!("  Avg Latency: {}ms", snapshot.average_latency_ms);
    println!("  Tokens Used: {}", snapshot.total_tokens_used);
    
    if snapshot.cache_hit_rate > 0.0 {
        println!("  Cache Hit Rate: {:.1}%", snapshot.cache_hit_rate);
    }
}
```

## Complete Example

Here's a full example using all new features:

```rust
use chatdelta::{
    create_client, ClientConfig, ClientMetrics, RequestTimer,
    http::get_provider_client
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize metrics
    let metrics = ClientMetrics::new();
    
    // Create client with optimized settings
    let config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .retry_strategy(RetryStrategy::Exponential(Duration::from_secs(1)))
        .build();
    
    let client = create_client("openai", &api_key, "gpt-4", config)?;
    
    // Make request with timing
    let timer = RequestTimer::new(metrics.clone());
    
    match client.send_prompt_with_metadata("Explain quantum computing").await {
        Ok(response) => {
            println!("Response: {}", response.content);
            
            // Record success with token count
            let tokens = response.metadata.total_tokens;
            timer.complete(true, Some(tokens));
        }
        Err(e) => {
            // Enhanced error message helps debugging
            eprintln!("Request failed: {}", e);
            timer.complete(false, None);
        }
    }
    
    // Display performance metrics
    let stats = metrics.get_stats();
    println!("\n📊 Session Metrics:");
    println!("  Success Rate: {:.1}%", stats.success_rate);
    println!("  Avg Latency: {}ms", stats.average_latency_ms);
    println!("  Total Tokens: {}", stats.total_tokens_used);
    
    Ok(())
}
```

## Performance Improvements

With v0.5.0, you can expect:

- **~30% reduction in latency** from connection pooling
- **Better error recovery** with enhanced retry strategies
- **Real-time performance visibility** through metrics
- **Reduced memory usage** from shared HTTP clients
- **Provider-optimized timeouts** for better reliability

## Troubleshooting

### Issue: Metrics not showing

Ensure you're using `ClientMetrics` and calling `record_request()`:

```rust
let metrics = ClientMetrics::new();
metrics.record_request(true, latency_ms, Some(tokens));
```

### Issue: Connection pool not working

Verify you're using the shared client infrastructure:

```rust
use chatdelta::http::SHARED_CLIENT;
// The client is automatically used by create_client()
```

### Issue: Old error messages

Update error handling to use the Display trait:

```rust
if let Err(e) = result {
    // This now shows enhanced error messages
    eprintln!("Error: {}", e);
}
```

## Need Help?

- Check the [examples](./examples/) directory for working code
- Review the [API documentation](https://docs.rs/chatdelta)
- Open an issue on [GitHub](https://github.com/ChatDelta/chatdelta-rs)