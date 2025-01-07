mod hyperliquid_spot_search_tool;
mod hyperliquid_perp_search_tool;

use hyperliquid_spot_search_tool::HyperliquidSpotSearchTool;
use hyperliquid_perp_search_tool::HyperliquidPerpSearchTool;
use rig::{completion::Prompt, providers::openai};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Create OpenAI client and model
    // This requires the `OPENAI_API_KEY` environment variable to be set.
    let openai_client = openai::Client::from_env();

    let gpt4 = openai_client.agent("gpt-4")
        .preamble("You are a helpful assistant that can search for cryptocurrency prices on Hyperliquid, most coins that are majors are on the perps platform, spot platform is only for coins on the hyperliquid platform")
        .tool(HyperliquidSpotSearchTool)
        .tool(HyperliquidPerpSearchTool)
        .build();

    // Original single-query version:
    /*
    let response = gpt4
        .prompt("What is the current BTC perpetual futures price on Hyperliquid?")
        .await?;

    let formatted_response: String = serde_json::from_str(&response)?;
    println!("Formatted response:\n{}", formatted_response);
    */

    // New interactive version:
    loop {
        print!("Enter your prompt (or 'quit' to exit): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        match gpt4.prompt(input).await {
            Ok(response) => {
                let formatted_response: String = serde_json::from_str(&response)?;
                println!("\nResponse:\n{}\n", formatted_response);
            },
            Err(e) => println!("Error: {}", e),
        }
    }

    Ok(())
}