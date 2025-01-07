use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest;
use rig::completion::ToolDefinition;
use rig::tool::Tool;

#[derive(Debug, Serialize, Deserialize)]
pub struct HyperliquidPerpArgs {
    symbol: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerpMarket {
    #[serde(rename = "szDecimals")]
    sz_decimals: i32,
    name: String,
    #[serde(rename = "maxLeverage")]
    max_leverage: i32,
    #[serde(rename = "onlyIsolated", default)]
    only_isolated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerpAssetContext {
    funding: String,
    #[serde(rename = "openInterest")]
    open_interest: String,
    #[serde(rename = "prevDayPx")]
    prev_day_px: String,
    #[serde(rename = "dayNtlVlm")]
    day_ntl_vlm: String,
    premium: Option<String>,
    #[serde(rename = "oraclePx")]
    oracle_px: String,
    #[serde(rename = "markPx")]
    mark_px: String,
    #[serde(rename = "midPx")]
    mid_px: Option<String>,
    #[serde(rename = "impactPxs")]
    impact_pxs: Option<Vec<String>>,
    #[serde(rename = "dayBaseVlm")]
    day_base_vlm: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerpMetaResponse {
    universe: Vec<PerpMarket>,
}

#[derive(Debug, thiserror::Error)]
pub enum HyperliquidPerpError {
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Invalid response structure")]
    InvalidResponse,
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
}

pub struct HyperliquidPerpSearchTool;

impl Tool for HyperliquidPerpSearchTool {
    const NAME: &'static str = "search_hyperliquid_perp";
    type Args = HyperliquidPerpArgs;
    type Output = String;
    type Error = HyperliquidPerpError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_hyperliquid_perp".to_string(),
            description: "Search for perpetual futures prices and data on Hyperliquid exchange".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Trading symbol to search for (e.g., 'BTC', 'ETH')"
                    }
                },
                "required": ["symbol"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        
        // Make request for perp metadata and asset contexts
        let url = "https://api.hyperliquid.xyz/info";
        
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&json!({
                "type": "metaAndAssetCtxs"
            }))
            .send()
            .await
            .map_err(|e| HyperliquidPerpError::HttpRequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(HyperliquidPerpError::ApiError(format!(
                "API returned status: {} - {}",
                status,
                error_text
            )));
        }

        // Parse the response as array
        let response_array: Vec<Value> = response
            .json()
            .await
            .map_err(|_| HyperliquidPerpError::InvalidResponse)?;

        if response_array.len() != 2 {
            return Err(HyperliquidPerpError::InvalidResponse);
        }

        // Extract the metadata and contexts
        let meta: PerpMetaResponse = serde_json::from_value(response_array[0].clone())
            .map_err(|_| HyperliquidPerpError::InvalidResponse)?;
        let contexts: Vec<PerpAssetContext> = serde_json::from_value(response_array[1].clone())
            .map_err(|_| HyperliquidPerpError::InvalidResponse)?;

        // Find the market index for the requested symbol
        let market_index = meta.universe
            .iter()
            .position(|market| market.name == args.symbol)
            .ok_or_else(|| HyperliquidPerpError::SymbolNotFound(args.symbol.clone()))?;

        // Get the corresponding context
        let context = &contexts[market_index];
        let market = &meta.universe[market_index];

        // Format the response
        let mut output = String::new();
        output.push_str(&format!("**{}** Perpetual Futures Information:\n\n", market.name));
        output.push_str(&format!("Mark Price: ${}\n", context.mark_px));
        if let Some(mid_px) = &context.mid_px {
            output.push_str(&format!("Mid Price: ${}\n", mid_px));
        }
        output.push_str(&format!("Oracle Price: ${}\n", context.oracle_px));
        output.push_str(&format!("Previous Day Price: ${}\n", context.prev_day_px));
        output.push_str(&format!("24h Volume: {}\n", context.day_base_vlm));
        output.push_str(&format!("Open Interest: {}\n", context.open_interest));
        output.push_str(&format!("Current Funding Rate: {}\n", context.funding));
        if let Some(premium) = &context.premium {
            output.push_str(&format!("Premium: {}\n", premium));
        }
        if let Some(impact_pxs) = &context.impact_pxs {
            if impact_pxs.len() >= 2 {
                output.push_str(&format!("Impact Prices (Buy/Sell): ${} / ${}\n", impact_pxs[0], impact_pxs[1]));
            }
        }
        output.push_str(&format!("Max Leverage: {}x\n", market.max_leverage));
        output.push_str(&format!("Size Decimals: {}\n", market.sz_decimals));
        output.push_str(&format!("Isolated Only: {}\n", market.only_isolated));

        Ok(output)
    }
} 