use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest;
use rig::completion::ToolDefinition;
use rig::tool::Tool;

/// Arguments required for the API call
/// Add all the fields your API endpoint needs
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateArgs {
    // Required fields should not have Option
    required_field: String,
    // Optional fields should use Option
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_field: Option<String>,
}

/// Response structure matching your API's JSON response
/// Use serde attributes to handle field naming differences
#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    // Example of renaming a field from snake_case to camelCase
    #[serde(rename = "someField")]
    some_field: String,
    
    // Example of an optional field
    #[serde(default)]
    optional_data: Option<String>,
    
    // Example of a nested structure
    #[serde(rename = "nestedData")]
    nested: NestedData,
}

/// Example of a nested data structure in the response
#[derive(Debug, Serialize, Deserialize)]
struct NestedData {
    // Example of a numeric field
    #[serde(rename = "numericValue")]
    numeric_value: f64,
    
    // Example of an array
    #[serde(rename = "arrayField")]
    array_field: Vec<String>,
}

/// Error types specific to your API
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Invalid response structure")]
    InvalidResponse,
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    // Add more error types as needed
}

/// The main tool struct
pub struct TemplateApiTool;

impl Tool for TemplateApiTool {
    // Define the tool's name - this should be unique
    const NAME: &'static str = "template_api_search";
    
    // Link the argument, output, and error types
    type Args = TemplateArgs;
    type Output = String;
    type Error = TemplateError;

    /// Define the tool's interface for the AI
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Description of what this tool does and when to use it".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "required_field": {
                        "type": "string",
                        "description": "Description of what this field is for"
                    },
                    "optional_field": {
                        "type": "string",
                        "description": "Description of this optional field"
                    }
                },
                "required": ["required_field"]
            }),
        }
    }

    /// Implement the actual API call and response handling
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Create an HTTP client
        let client = reqwest::Client::new();
        
        // Build the API request
        let url = "https://api.example.com/endpoint";
        
        // Example of a POST request with JSON body
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            // Add any required headers
            // .header("Authorization", "Bearer YOUR_TOKEN")
            .json(&json!({
                "field": args.required_field,
                "optionalField": args.optional_field
            }))
            .send()
            .await
            .map_err(|e| TemplateError::HttpRequestFailed(e.to_string()))?;

        // Handle non-200 responses
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(TemplateError::ApiError(format!(
                "API returned status: {} - {}",
                status,
                error_text
            )));
        }

        // Parse the response
        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|_| TemplateError::InvalidResponse)?;

        // Format the output
        let mut output = String::new();
        output.push_str(&format!("Field: {}\n", api_response.some_field));
        
        if let Some(optional) = api_response.optional_data {
            output.push_str(&format!("Optional: {}\n", optional));
        }
        
        output.push_str(&format!("Numeric Value: {}\n", api_response.nested.numeric_value));
        output.push_str("Array Values:\n");
        for item in api_response.nested.array_field {
            output.push_str(&format!("- {}\n", item));
        }

        Ok(output)
    }
}

// Optional: Add tests for your tool
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_call() {
        let tool = TemplateApiTool;
        let args = TemplateArgs {
            required_field: "test".to_string(),
            optional_field: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());
    }
} 