use chatdelta::{create_client, ClientConfig, AiClient};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure client
    let config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .temperature(0.7)
        .max_tokens(100)
        .build();

    let test_prompt = "Please respond with exactly: 'Hello from [your model name]!' where you replace [your model name] with your actual model name.";

    // Test OpenAI if API key exists
    if let Ok(openai_key) = env::var("OPENAI_API_KEY") {
        println!("Testing OpenAI...");
        let client = create_client("openai", &openai_key, "gpt-4o-mini", config.clone())?;
        match client.send_prompt(test_prompt).await {
            Ok(response) => println!("OpenAI response: {}", response),
            Err(e) => eprintln!("OpenAI error: {}", e),
        }
    } else if let Ok(openai_key) = env::var("CHATGPT_API_KEY") {
        println!("Testing OpenAI...");
        let client = create_client("openai", &openai_key, "gpt-4o-mini", config.clone())?;
        match client.send_prompt(test_prompt).await {
            Ok(response) => println!("OpenAI response: {}", response),
            Err(e) => eprintln!("OpenAI error: {}", e),
        }
    } else {
        println!("OPENAI_API_KEY or CHATGPT_API_KEY not found - skipping OpenAI test");
    }

    println!();

    // Test Google Gemini if API key exists
    if let Ok(gemini_key) = env::var("GEMINI_API_KEY") {
        println!("Testing Google Gemini...");
        let client = create_client("gemini", &gemini_key, "gemini-1.5-flash", config.clone())?;
        match client.send_prompt(test_prompt).await {
            Ok(response) => println!("Gemini response: {}", response),
            Err(e) => eprintln!("Gemini error: {}", e),
        }
    } else {
        println!("GEMINI_API_KEY not found - skipping Gemini test");
    }

    println!();

    // Test Anthropic Claude if API key exists
    if let Ok(claude_key) = env::var("CLAUDE_API_KEY") {
        println!("Testing Anthropic Claude...");
        let client = create_client("claude", &claude_key, "claude-3-haiku-20240307", config.clone())?;
        match client.send_prompt(test_prompt).await {
            Ok(response) => println!("Claude response: {}", response),
            Err(e) => eprintln!("Claude error: {}", e),
        }
    } else if let Ok(anthropic_key) = env::var("ANTHROPIC_API_KEY") {
        println!("Testing Anthropic Claude...");
        let client = create_client("claude", &anthropic_key, "claude-3-haiku-20240307", config.clone())?;
        match client.send_prompt(test_prompt).await {
            Ok(response) => println!("Claude response: {}", response),
            Err(e) => eprintln!("Claude error: {}", e),
        }
    } else {
        println!("CLAUDE_API_KEY or ANTHROPIC_API_KEY not found - skipping Claude test");
    }

    Ok(())
}