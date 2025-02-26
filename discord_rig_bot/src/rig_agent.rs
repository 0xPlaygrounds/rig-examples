// rig_agent.rs

use anyhow::{Context, Result};
use rig::{
    agent::Agent, completion::Prompt, embeddings::EmbeddingsBuilder, providers::openai,
    vector_store::in_memory_store::InMemoryVectorStore,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use serenity::client::Context as SerenityContext;
use serenity::model::channel::Message;

pub struct RigAgent {
    agent: Arc<Agent<openai::CompletionModel>>,
}

impl RigAgent {
    pub async fn new() -> Result<Self> {
        // Initialize OpenAI client
        let openai_client = openai::Client::from_env();
        let embedding_model = openai_client.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);

        // Create vector store
        let mut vector_store = InMemoryVectorStore::default();

        // Get the current directory and construct paths to markdown files
        let current_dir = std::env::current_dir()?;
        let documents_dir = current_dir.join("documents");

        let md1_path = documents_dir.join("Rig_guide.md");
        let md2_path = documents_dir.join("Rig_faq.md");
        let md3_path = documents_dir.join("Rig_examples.md");

        // Load markdown documents
        let md1_content = Self::load_md_content(&md1_path)?;
        let md2_content = Self::load_md_content(&md2_path)?;
        let md3_content = Self::load_md_content(&md3_path)?;

        //Create embeddings add to vector store
        let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
            .document(md1_content)?
            .document(md2_content)?
            .document(md3_content)?
            .build()
            .await?;

        vector_store.add_documents(embeddings);

        // Create index
        let index = vector_store.index(embedding_model);

        // Create Agent
        let agent = Arc::new(
            openai_client
                .agent(openai::GPT_4O)
                .preamble(
                    "You are an advanced AI assistant powered by Rig, a Rust library for building LLM applications. Your primary function is to provide accurate, helpful, and context-aware responses by leveraging both your general knowledge and specific information retrieved from a curated knowledge base.

                    Key responsibilities and behaviors:
                    1. Information Retrieval: You have access to a vast knowledge base. When answering questions, always consider the context provided by the retrieved information.
                    2. Clarity and Conciseness: Provide clear and concise answers. Ensure responses are short and concise. Use bullet points or numbered lists for complex information when appropriate.
                    3. Technical Proficiency: You have deep knowledge about Rig and its capabilities. When discussing Rig or answering related questions, provide detailed and technically accurate information.
                    5. Code Examples: When appropriate, provide Rust code examples to illustrate concepts, especially when discussing Rig's functionalities. Always format code examples for proper rendering in Discord by wrapping them in triple backticks and specifying the language as 'rust'. For example:
                        ```rust
                        let example_code = \"This is how you format Rust code for Discord\";
                        println!(\"{}\", example_code);
                        ```
                ",
                )
                .dynamic_context(2, index)
                .build(),
    );

        Ok(Self { agent })
    }

    fn load_md_content<P: AsRef<Path>>(file_path: P) -> Result<String> {
        fs::read_to_string(file_path.as_ref())
            .with_context(|| format!("Failed to read markdown file: {:?}", file_path.as_ref()))
    }
    
    // Add this function for messages that only need a string input/output
    pub async fn process_string(&self, message: &str) -> Result<String> {
        self.agent
            .prompt(message)
            .await
            .map_err(anyhow::Error::from)
    }
    
    pub async fn process_message(&self, ctx: &SerenityContext, msg: &Message) -> Result<String> {
        // First, create a typing indicator
        msg.channel_id.broadcast_typing(&ctx.http).await?;
        
        // Send deferred response to meet 3-second requirement
        let mut deferred_msg = msg.channel_id.say(&ctx.http, "Thinking...").await?;
        
        // Use the string content directly, not a reference
        let response = self.agent.prompt(msg.content.clone()).await.map_err(anyhow::Error::from)?;
        
        // Truncate if needed
        let truncated_response = if response.len() > 1900 {
            format!("Response truncated due to Discord limits:\n{}", &response[..1897])
        } else {
            response
        };
        
        // Edit the deferred message
        deferred_msg.edit(&ctx.http, |m| m.content(truncated_response.clone())).await?;
        
        Ok(truncated_response)
    }

    // OLD process_message WITHOUT DEFERRAL AND TRUNCATION
    // pub async fn process_message(&self, message: &str) -> Result<String> {
    //     self.agent
    //         .prompt(message)
    //         .await
    //         .map_err(anyhow::Error::from)
    // }
}
