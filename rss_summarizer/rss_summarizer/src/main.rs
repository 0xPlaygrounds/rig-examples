use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest;
use rig::providers::openai;
use rss::Channel;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::time::{self, Duration};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct SummarizedRssItem {
    title: String,
    link: String,
    #[schemars(with = "String")]
    pub_date: DateTime<Utc>,
    summary: String,
    relevance_score: f32,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct RssSummary {
    items: Vec<SummarizedRssItem>,
    total_count: usize,
    extraction_time: String, // ISO 8601 formatted string
    overall_summary: String,
}

fn pretty_print_summary(summary: &RssSummary) {
    println!("RSS Feed Summary:");
    println!("Total Items: {}", summary.total_count);
    println!("Extraction Time: {}", summary.extraction_time);
    println!("\nTop Items:");
    for (i, item) in summary.items.iter().enumerate() {
        println!("{}. {}", i + 1, item.title);
        println!("   Link: {}", item.link);
        println!("   Published: {}", item.pub_date);
        println!("   Summary: {}", item.summary);
        println!("   Relevance Score: {:.2}", item.relevance_score);
        println!();
    }
    println!("Overall Summary: {}", summary.overall_summary);
}

async fn fetch_rss_feed(url: &str) -> Result<Channel, Box<dyn Error>> {
    let response = reqwest::get(url).await?.text().await?;
    let channel = response.parse::<Channel>()?;
    Ok(channel)
}

fn sanitize_string(input: &str) -> String {
    let mut sanitized = input.to_string();
    sanitized = sanitized.replace("\n", " ");
    sanitized = sanitized.replace("\r", "");
    sanitized = sanitized.replace("\"", "");
    sanitized = sanitized.replace("’", "'"); // Replace any special quotes
    sanitized
}

async fn summarize_rss_feed(channel: Channel) -> Result<RssSummary, Box<dyn Error>> {
    // Initialize the OpenAI client
    let openai_client = openai::Client::from_env();

    // Create the extractor
    let extractor = openai_client
        .extractor::<RssSummary>(openai::GPT_4)
        .preamble(
            "You are an AI assistant specialized in summarizing RSS feeds. \
                   Your task is to analyze the RSS items, extract the most relevant information, \
                   and provide concise summaries. For each item, provide a brief summary and a \
                   relevance score from 0.0 to 1.0. Also, provide an overall summary of the feed.",
        )
        .build();

    // Convert RSS items to a format suitable for summarization
    let rss_items = channel.items();
    let mut formatted_rss = String::new();

    // Create regex to remove HTML tags and CDATA sections
    let re_html = Regex::new(r"(?i)<[^>]*>").unwrap();
    let re_cdata = Regex::new(r"(?i)<!\[CDATA\[.*?\]\]>").unwrap();

    for (i, item) in rss_items.iter().enumerate() {
        let title = item.title().unwrap_or("").to_string();
        let link = item.link().unwrap_or("").to_string();
        let pub_date = item.pub_date().unwrap_or("").to_string();
        let description = item.description().unwrap_or("").to_string();

        // Remove CDATA sections and HTML tags
        let clean_description = re_html
            .replace_all(&re_cdata.replace_all(&description, ""), "")
            .to_string();
        let sanitized_description = sanitize_string(&clean_description);

        formatted_rss.push_str(&format!(
            "{}. Title: {}\nLink: {}\nDate: {}\nDescription: {}\n\n",
            i + 1,
            sanitize_string(&title),
            sanitize_string(&link),
            sanitize_string(&pub_date),
            sanitized_description
        ));
    }

    println!("Extracting summary from the RSS feed...\n");

    // Extract summary
    let rss_summary = extractor.extract(&formatted_rss).await?;

    Ok(rss_summary)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let rss_url = "https://news.ycombinator.com/rss";
    let mut interval = time::interval(Duration::from_secs(3600)); // 1 hour interval

    loop {
        interval.tick().await;

        match fetch_rss_feed(rss_url).await {
            Ok(channel) => match summarize_rss_feed(channel).await {
                Ok(rss_summary) => {
                    pretty_print_summary(&rss_summary);
                }
                Err(e) => eprintln!("Error summarizing RSS feed: {}", e),
            },
            Err(e) => eprintln!("Error fetching RSS feed: {}", e),
        }
    }
}
