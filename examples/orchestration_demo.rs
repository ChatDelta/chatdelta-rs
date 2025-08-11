//! Demonstration of AI Orchestration capabilities

use chatdelta::{
    AiOrchestrator, OrchestrationStrategy, PromptOptimizer,
    create_client, ClientConfig,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ChatDelta AI Orchestration Demo\n");
    
    // Initialize AI clients
    let config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .build();
    
    let mut clients = Vec::new();
    
    // Add available AI models
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        clients.push(create_client("openai", &key, "gpt-4", config.clone())?);
        println!("✅ Added GPT-4");
    }
    
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        clients.push(create_client("claude", &key, "claude-3-opus", config.clone())?);
        println!("✅ Added Claude 3 Opus");
    }
    
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        clients.push(create_client("gemini", &key, "gemini-1.5-pro", config.clone())?);
        println!("✅ Added Gemini 1.5 Pro");
    }
    
    if clients.is_empty() {
        eprintln!("❌ No AI clients configured. Set API keys to continue.");
        return Ok(());
    }
    
    // Create orchestrator
    let orchestrator = AiOrchestrator::new(clients)
        .with_strategy(OrchestrationStrategy::WeightedFusion);
    
    println!("\n📋 Orchestration Strategy: Weighted Fusion");
    
    // Create prompt optimizer
    let optimizer = PromptOptimizer::new();
    
    // Example queries
    let queries = vec![
        "Explain quantum computing in simple terms",
        "Write a haiku about artificial intelligence",
        "What are the best practices for Rust error handling?",
    ];
    
    for query in queries {
        println!("\n{'='*60}");
        println!("📝 Original Query: {}", query);
        
        // Optimize the prompt
        let optimized = optimizer.optimize(query);
        println!("🔧 Optimized Query: {}", optimized.optimized);
        println!("   Techniques Applied: {:?}", optimized.techniques_applied);
        println!("   Confidence: {:.1}%", optimized.confidence * 100.0);
        
        // Execute orchestrated query
        println!("\n🎭 Orchestrating AI responses...");
        
        match orchestrator.query(&optimized.optimized).await {
            Ok(response) => {
                println!("\n✨ Fused Response (Confidence: {:.1}%):", response.confidence * 100.0);
                println!("{}", response.content);
                
                println!("\n📊 Model Contributions:");
                for contribution in &response.contributions {
                    println!("   {} - Weight: {:.2}, Confidence: {:.1}%, Latency: {}ms",
                        contribution.model,
                        contribution.weight,
                        contribution.confidence * 100.0,
                        contribution.latency_ms
                    );
                }
                
                println!("\n🤝 Consensus Analysis:");
                println!("   Agreement Score: {:.1}%", response.consensus.agreement_score * 100.0);
                
                println!("\n⚡ Performance Metrics:");
                println!("   Total Latency: {}ms", response.metrics.total_latency_ms);
                println!("   Models Used: {}", response.metrics.models_used);
                println!("   Estimated Cost: ${:.4}", response.metrics.cost_estimate);
            }
            Err(e) => {
                eprintln!("❌ Orchestration failed: {}", e);
            }
        }
    }
    
    // Demonstrate different strategies
    println!("\n\n🎯 Testing Different Orchestration Strategies:");
    
    let strategies = vec![
        OrchestrationStrategy::Parallel,
        OrchestrationStrategy::Tournament,
        OrchestrationStrategy::Consensus,
    ];
    
    let test_prompt = "What is the meaning of life?";
    
    for strategy in strategies {
        println!("\n📍 Strategy: {:?}", strategy);
        
        let orchestrator = AiOrchestrator::new(clients.clone())
            .with_strategy(strategy);
        
        if let Ok(response) = orchestrator.query(test_prompt).await {
            println!("   Result Preview: {}...", 
                &response.content.chars().take(100).collect::<String>());
            println!("   Confidence: {:.1}%", response.confidence * 100.0);
        }
    }
    
    println!("\n🎉 Demo Complete!");
    
    Ok(())
}