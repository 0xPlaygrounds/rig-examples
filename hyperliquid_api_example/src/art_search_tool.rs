use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

// 1. First, let's define our input arguments structure
#[derive(Deserialize)]
pub struct ArtSearchArgs {
    // Required
    query: String,
    // Optional parameters
    limit: Option<u32>,
    page: Option<u32>,
    fields: Option<String>,
    sort: Option<String>,
}

// 2. Define the artwork response structure
#[derive(Deserialize, Serialize)]
pub struct Artwork {
    id: String,
    title: String,
    artist_display: Option<String>,
    date_display: Option<String>,
    medium_display: Option<String>,
    dimensions: Option<String>,
    image_id: Option<String>,
    thumbnail: Option<serde_json::Value>,
    description: Option<String>,
}

// 3. Define possible errors
#[derive(Debug, thiserror::Error)]
pub enum ArtSearchError {
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),
    #[error("Invalid response structure")]
    InvalidResponse,
    #[error("API error: {0}")]
    ApiError(String),
}

pub struct ArtSearchTool;

impl Tool for ArtSearchTool {
    const NAME: &'static str = "search_art";
    type Args = ArtSearchArgs;
    type Output = String;
    type Error = ArtSearchError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_art".to_string(),
            description: "Search for artworks in the Art Institute of Chicago collection".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search term for artwork"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of results to return (default: 10)"
                    },
                    "page": {
                        "type": "integer",
                        "description": "Page number for pagination"
                    },
                    "fields": {
                        "type": "string",
                        "description": "Comma-separated list of fields to return"
                    },
                    "sort": {
                        "type": "string",
                        "description": "Sort order (e.g., '_score', 'title')"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let client = reqwest::Client::new();
        
        // Use the correct endpoint for searching artworks
        let mut url = format!(
            "https://api.artic.edu/api/v1/artworks?fields=id,title,artist_display,date_display,description&q={}",
            args.query
        );

        // Add optional parameters
        if let Some(limit) = args.limit {
            url.push_str(&format!("&limit={}", limit));
        }
        if let Some(page) = args.page {
            url.push_str(&format!("&page={}", page));
        }
        if let Some(sort) = args.sort {
            url.push_str(&format!("&sort={}", sort));
        }

        println!("Requesting URL: {}", url); // Debug print

        // Make the API request
        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Origin", "https://api.artic.edu")
            .header("Referer", "https://api.artic.edu/")
            .send()
            .await
            .map_err(|e| ArtSearchError::HttpRequestFailed(e.to_string()))?;

        // Debug print the response status
        println!("Response status: {}", response.status());

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();  // Get status first
            let error_text = response.text().await.unwrap_or_default();
            return Err(ArtSearchError::ApiError(format!(
                "API returned status: {} - {}",
                status,
                error_text
            )));
        }

        // Parse the response
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ArtSearchError::InvalidResponse)?;

        // Format the results
        let mut output = String::new();
        
        if let Some(data) = data.get("data") {
            if let Some(artworks) = data.as_array() {
                output.push_str("Found artworks:\n\n");
                
                for (i, artwork) in artworks.iter().enumerate() {
                    let title = artwork.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                    let artist = artwork
                        .get("artist_display")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Artist");
                    
                    output.push_str(&format!("{}. **{}**\n", i + 1, title));
                    output.push_str(&format!("   Artist: {}\n", artist));
                    
                    if let Some(date) = artwork.get("date_display").and_then(|v| v.as_str()) {
                        output.push_str(&format!("   Date: {}\n", date));
                    }
                    
                    if let Some(desc) = artwork.get("description").and_then(|v| v.as_str()) {
                        output.push_str(&format!("   Description: {}\n", desc));
                    }
                    
                    output.push_str("\n");
                }
            }
        }

        if output.is_empty() {
            output = "No artworks found.".to_string();
        }

        Ok(output)
    }
}