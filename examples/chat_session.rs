//! Example demonstrating multi-turn conversations with ChatSession

use chatdelta::{create_client, ChatSession, ClientConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the client with a system message
    let config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .temperature(0.7)
        .system_message("You are a helpful coding assistant specializing in Rust.")
        .build();

    // Create a client
    let client = create_client(
        "openai",
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4",
        config,
    )?;

    // Create a chat session
    let mut session = ChatSession::new(Box::new(client));

    // First message
    println!("User: What are the main benefits of Rust's ownership system?");
    let response = session
        .send("What are the main benefits of Rust's ownership system?")
        .await?;
    println!("Assistant: {}\n", response);

    // Follow-up question (maintains context)
    println!("User: Can you give me a code example of the first benefit?");
    let response = session
        .send("Can you give me a code example of the first benefit?")
        .await?;
    println!("Assistant: {}\n", response);

    // Another follow-up with metadata
    println!("User: How does this compare to garbage collection?");
    let response_with_metadata = session
        .send_with_metadata("How does this compare to garbage collection?")
        .await?;
    println!("Assistant: {}", response_with_metadata.content);
    
    // Show conversation stats
    println!("\n--- Conversation Stats ---");
    println!("Messages in history: {}", session.history().len());
    if let Some(tokens) = response_with_metadata.metadata.total_tokens {
        println!("Tokens used in last response: {}", tokens);
    }

    // Clear conversation and start fresh with a new topic
    session.reset_with_system("You are a helpful assistant specializing in web development.");
    
    println!("\n--- New Conversation ---");
    println!("User: What's the difference between REST and GraphQL?");
    let response = session
        .send("What's the difference between REST and GraphQL?")
        .await?;
    println!("Assistant: {}", response);

    Ok(())
}