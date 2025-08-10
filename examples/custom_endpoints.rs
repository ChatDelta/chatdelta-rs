//! Example demonstrating custom API endpoints (e.g., Azure OpenAI, local models)

use chatdelta::{create_client, ClientConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Azure OpenAI endpoint
    println!("--- Azure OpenAI Example ---");
    let azure_config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .base_url("https://your-resource.openai.azure.com/openai/deployments/your-deployment")
        .temperature(0.7)
        .build();

    let azure_client = create_client(
        "openai",
        std::env::var("AZURE_OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4", // Model name (deployment name for Azure)
        azure_config,
    )?;

    // Note: This would work with a real Azure endpoint
    // let response = azure_client.send_prompt("Hello from Azure!").await?;
    // println!("Azure Response: {}", response);

    // Example 2: Local model server (e.g., Ollama, LocalAI, or custom server)
    println!("\n--- Local Model Example ---");
    let local_config = ClientConfig::builder()
        .timeout(Duration::from_secs(60)) // Longer timeout for local models
        .base_url("http://localhost:11434/v1") // Example: Ollama with OpenAI-compatible API
        .temperature(0.8)
        .max_tokens(500)
        .build();

    let local_client = create_client(
        "openai", // Use OpenAI client for OpenAI-compatible APIs
        "no-key-needed-for-local", // Local servers often don't need API keys
        "llama2", // Or any model available on your local server
        local_config,
    )?;

    // Note: This would work with a real local server
    // let response = local_client.send_prompt("Tell me a joke about Rust programming").await?;
    // println!("Local Model Response: {}", response);

    // Example 3: Custom proxy or gateway
    println!("\n--- API Gateway Example ---");
    let gateway_config = ClientConfig::builder()
        .timeout(Duration::from_secs(45))
        .base_url("https://api-gateway.company.com/ai/v1")
        .temperature(0.7)
        .retries(3) // Retry on gateway errors
        .build();

    let gateway_client = create_client(
        "openai",
        std::env::var("COMPANY_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        "gpt-4",
        gateway_config,
    )?;

    // Note: This would work with a real gateway
    // let response = gateway_client.send_prompt("Hello through the gateway!").await?;
    // println!("Gateway Response: {}", response);

    println!("\nNote: These examples show how to configure custom endpoints.");
    println!("To run them, you'll need to set up the corresponding services and API keys.");

    Ok(())
}