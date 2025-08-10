//! Example demonstrating different retry strategies for handling failures

use chatdelta::{create_client, ClientConfig, RetryStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Fixed retry strategy
    // Retries with a constant 2-second delay between attempts
    println!("--- Fixed Retry Strategy ---");
    let fixed_config = ClientConfig::builder()
        .timeout(Duration::from_secs(10))
        .retries(3)
        .retry_strategy(RetryStrategy::Fixed(Duration::from_secs(2)))
        .build();

    let client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        fixed_config,
    )?;
    println!("Client configured with fixed 2s delay between retries");

    // Example 2: Linear retry strategy
    // Delay increases linearly: 1s, 2s, 3s, 4s...
    println!("\n--- Linear Retry Strategy ---");
    let linear_config = ClientConfig::builder()
        .timeout(Duration::from_secs(10))
        .retries(3)
        .retry_strategy(RetryStrategy::Linear(Duration::from_secs(1)))
        .build();

    let client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        linear_config,
    )?;
    println!("Client configured with linear backoff (1s, 2s, 3s...)");

    // Example 3: Exponential backoff (default)
    // Delay doubles each time: 1s, 2s, 4s, 8s...
    println!("\n--- Exponential Backoff Strategy ---");
    let exponential_config = ClientConfig::builder()
        .timeout(Duration::from_secs(10))
        .retries(4)
        .retry_strategy(RetryStrategy::Exponential(Duration::from_secs(1)))
        .build();

    let client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        exponential_config,
    )?;
    println!("Client configured with exponential backoff (1s, 2s, 4s, 8s...)");

    // Example 4: Exponential backoff with jitter
    // Adds randomization to prevent thundering herd problem
    println!("\n--- Exponential Backoff with Jitter ---");
    let jitter_config = ClientConfig::builder()
        .timeout(Duration::from_secs(10))
        .retries(4)
        .retry_strategy(RetryStrategy::ExponentialWithJitter(Duration::from_millis(500)))
        .build();

    let client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        jitter_config,
    )?;
    println!("Client configured with exponential backoff + 0-30% jitter");

    // Example 5: Aggressive retry for critical operations
    println!("\n--- Aggressive Retry for Critical Operations ---");
    let aggressive_config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .retries(5) // More retry attempts
        .retry_strategy(RetryStrategy::Fixed(Duration::from_millis(500))) // Short delays
        .build();

    let critical_client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        aggressive_config,
    )?;
    println!("Client configured for critical operations: 5 retries with 500ms delays");

    // Example 6: Conservative retry for rate-limited APIs
    println!("\n--- Conservative Retry for Rate-Limited APIs ---");
    let conservative_config = ClientConfig::builder()
        .timeout(Duration::from_secs(60))
        .retries(3)
        .retry_strategy(RetryStrategy::Exponential(Duration::from_secs(5))) // Longer base delay
        .build();

    let rate_limited_client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        conservative_config,
    )?;
    println!("Client configured for rate-limited APIs: exponential backoff starting at 5s");

    println!("\n--- Retry Strategy Benefits ---");
    println!("• Fixed: Predictable delays, good for temporary network issues");
    println!("• Linear: Gradual increase, balanced approach");
    println!("• Exponential: Reduces load on struggling servers");
    println!("• Exponential with Jitter: Prevents synchronized retries from multiple clients");

    Ok(())
}