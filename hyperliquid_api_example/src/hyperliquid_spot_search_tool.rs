use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct HyperliquidSpotArgs {
    // Required
    symbol: String,
}

#[derive(Deserialize, Serialize)]
pub struct Token {
    name: String,
    #[serde(rename = "szDecimals")]
    sz_decimals: i32,
    #[serde(rename = "weiDecimals")]
    wei_decimals: i32,
    index: i32,
    #[serde(rename = "tokenId")]
    token_id: String,
    #[serde(rename = "isCanonical")]
    is_canonical: bool,
    #[serde(rename = "evmContract")]
    evm_contract: Option<String>,
    #[serde(rename = "fullName")]
    full_name: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Market {
    name: String,
    tokens: Vec<i32>,
    index: i32,
    #[serde(rename = "isCanonical")]
    is_canonical: bool,
}

#[derive(Deserialize, Serialize)]
pub struct AssetContext {
    #[serde(rename = "dayNtlVlm")]
    day_ntl_vlm: String,
    #[serde(rename = "markPx")]
    mark_px: String,
    #[serde(rename = "midPx")]
    mid_px: Option<String>,
    #[serde(rename = "prevDayPx")]
    prev_day_px: String,
    coin: String,
    #[serde(rename = "circulatingSupply")]
    circulating_supply: String,
    #[serde(rename = "totalSupply")]
    total_supply: String,
    #[serde(rename = "dayBaseVlm")]
    day_base_vlm: String,
}

#[derive(Deserialize, Serialize)]
pub struct SpotMetaResponse {
    tokens: Vec<Token>,
    universe: Vec<Market>,
}

#[derive(Debug, thiserror::Error)]
pub enum HyperliquidSpotError {
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),
    #[error("Invalid response structure")]
    InvalidResponse,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
}

pub struct HyperliquidSpotSearchTool;

impl Tool for HyperliquidSpotSearchTool {
    const NAME: &'static str = "search_hyperliquid_spot";
    type Args = HyperliquidSpotArgs;
    type Output = String;
    type Error = HyperliquidSpotError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_hyperliquid_spot".to_string(),
            description: "Search for spot prices on Hyperliquid exchange".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Trading symbol to search for (e.g., 'PURR', 'SPH')"
                    }
                },
                "required": ["symbol"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        
        // Make request for spot metadata and asset contexts
        let url = "https://api.hyperliquid.xyz/info";
        
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&json!({
                "type": "spotMetaAndAssetCtxs"
            }))
            .send()
            .await
            .map_err(|e| HyperliquidSpotError::HttpRequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(HyperliquidSpotError::ApiError(format!(
                "API returned status: {} - {}",
                status,
                error_text
            )));
        }

        // Parse the response
        let response_array: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|_| HyperliquidSpotError::InvalidResponse)?;

        // Extract the metadata and contexts
        let meta: SpotMetaResponse = serde_json::from_value(response_array[0].clone())
            .map_err(|_| HyperliquidSpotError::InvalidResponse)?;
        let contexts: Vec<AssetContext> = serde_json::from_value(response_array[1].clone())
            .map_err(|_| HyperliquidSpotError::InvalidResponse)?;

        // First try to find the token in metadata by name
        let token = meta.tokens.iter()
            .find(|t| t.name == args.symbol.to_uppercase())
            .ok_or_else(|| HyperliquidSpotError::SymbolNotFound(args.symbol.clone()))?;

        // Find the market that uses this token
        let market = meta.universe.iter()
            .find(|m| m.name.split('/').next().unwrap_or("") == token.name)
            .ok_or_else(|| HyperliquidSpotError::SymbolNotFound(args.symbol.clone()))?;

        // Find the context using the market name
        let context = contexts.iter()
            .find(|c| c.coin == market.name)
            .ok_or_else(|| HyperliquidSpotError::SymbolNotFound(args.symbol.clone()))?;

        // Format the output
        let mut output = String::new();
        output.push_str(&format!("**{}** Spot Information:\n\n", token.name));
        output.push_str(&format!("Mark Price: ${}\n", context.mark_px));
        if let Some(mid_px) = &context.mid_px {
            output.push_str(&format!("Mid Price: ${}\n", mid_px));
        }
        output.push_str(&format!("Previous Day Price: ${}\n", context.prev_day_px));
        output.push_str(&format!("24h Volume: ${}\n", context.day_ntl_vlm));
        output.push_str(&format!("24h Base Volume: {}\n", context.day_base_vlm));
        output.push_str(&format!("Circulating Supply: {}\n", context.circulating_supply));
        output.push_str(&format!("Total Supply: {}\n", context.total_supply));

        if let Some(full_name) = &token.full_name {
            output.push_str(&format!("Full Name: {}\n", full_name));
        }

        Ok(output)
    }
}