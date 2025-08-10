//! Example demonstrating token usage reporting and response metadata

use chatdelta::{create_client, ClientConfig, AiResponse};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the client
    let config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .temperature(0.7)
        .max_tokens(150)
        .build();

    // Create an OpenAI client
    let client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4",
        config,
    )?;

    // Send a prompt and get response with metadata
    let response: AiResponse = client
        .send_prompt_with_metadata("Explain quantum computing in one sentence.")
        .await?;

    println!("Response: {}", response.content);
    println!("\n--- Metadata ---");
    
    if let Some(model) = &response.metadata.model_used {
        println!("Model used: {}", model);
    }
    
    if let Some(prompt_tokens) = response.metadata.prompt_tokens {
        println!("Prompt tokens: {}", prompt_tokens);
    }
    
    if let Some(completion_tokens) = response.metadata.completion_tokens {
        println!("Completion tokens: {}", completion_tokens);
    }
    
    if let Some(total_tokens) = response.metadata.total_tokens {
        println!("Total tokens: {}", total_tokens);
        
        // Estimate cost (example rates, adjust based on actual pricing)
        let cost_per_1k_prompt = 0.03; // $0.03 per 1K tokens for GPT-4
        let cost_per_1k_completion = 0.06; // $0.06 per 1K tokens for GPT-4
        
        let prompt_cost = (prompt_tokens.unwrap_or(0) as f64 / 1000.0) * cost_per_1k_prompt;
        let completion_cost = (completion_tokens.unwrap_or(0) as f64 / 1000.0) * cost_per_1k_completion;
        let total_cost = prompt_cost + completion_cost;
        
        println!("Estimated cost: ${:.4}", total_cost);
    }
    
    if let Some(finish_reason) = &response.metadata.finish_reason {
        println!("Finish reason: {}", finish_reason);
    }
    
    if let Some(latency_ms) = response.metadata.latency_ms {
        println!("Response time: {}ms", latency_ms);
    }

    Ok(())
}