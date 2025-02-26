// main.rs

mod rig_agent;

use anyhow::Result;
use dotenv::dotenv;
use rig_agent::RigAgent;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::command::CommandOptionType;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};

// Define a key for storing the bot's user ID in the TypeMap
struct BotUserId;

impl TypeMapKey for BotUserId {
    type Value = serenity::model::id::UserId;
}

struct Handler {
    rig_agent: Arc<RigAgent>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        debug!("\n\n======> Received an interaction");
        if let Interaction::ApplicationCommand(command) = interaction {
            debug!("\n\n======> Received command: {}", command.data.name);
            
            match command.data.name.as_str() {
                "hello" => {
                    let content = "Hello! I'm your helpful Rust and Rig-powered assistant. How can I assist you today?".to_string();
                    
                    if let Err(why) = command
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message.content(content))
                        })
                        .await
                    {
                        error!("Cannot respond to slash command: {}", why);
                    }
                },
                "ask" => {
                    // Step 1: Acknowledge quickly
                    if let Err(e) = command
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                        })
                        .await
                    {
                        error!("Failed to create deferred response: {:?}", e);
                        return;
                    }
    
                    let query = command
                        .data
                        .options
                        .get(0)
                        .and_then(|opt| opt.value.as_ref())
                        .and_then(|v| v.as_str())
                        .unwrap_or("What would you like to ask?");
                    
                    debug!("\n\n======> Query: {}", query);
                    
                    let response = match self.rig_agent.process_string(query).await {
                        Ok(response) => {
                            if response.len() > 1900 {
                                format!("Response truncated due to Discord limits:\n{}", &response[..1897])
                            } else {
                                response
                            }
                        },
                        Err(e) => {
                            error!("Error processing request: {:?}", e);
                            format!("Error processing request: {:?}", e)
                        }
                    };
                    
                    // Step 3: Edit the original response
                    if let Err(e) = command
                        .edit_original_interaction_response(&ctx.http, |message| {
                            message.content(response)
                        })
                        .await
                    {
                        error!("Failed to edit interaction response: {:?}", e);
                    }
                },
                _ => {
                    if let Err(why) = command
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| 
                                    message.content("Not implemented :("))
                        })
                        .await
                    {
                        error!("Cannot respond to slash command: {}", why);
                    }
                }
            }
            
            debug!("\n\n======> Response sent successfully");
        }
    }
    
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.mentions_me(&ctx.http).await.unwrap_or(false) {
            debug!("\n\n=====> Bot mentioned in message: {}", msg.content);

            let bot_id = {
                let data = ctx.data.read().await;
                data.get::<BotUserId>().copied()
            };

            if let Some(bot_id) = bot_id {
                let mention = format!("<@{}>", bot_id);
                let content = msg.content.replace(&mention, "").trim().to_string();

                debug!(
                    "\n\n=====> Processed content after removing mention: {}",
                    content
                );

                match self.rig_agent.process_message(&ctx, &msg).await {
                    Ok(response) => {
                        println!("Response sent successfully.");
                        println!("{}", response);
                    }
                    Err(e) => {
                        println!("Error processing request: {:?}", e);
                        if let Err(why) = msg.channel_id.say(&ctx.http, format!("Error processing request: {:?}", e)).await {
                            println!("Error sending error message: {:?}", why);
                        }
                    }
                }

                match self.rig_agent.process_message(&ctx, &msg).await {
                    Ok(response) => {
                        println!("Response sent successfully.");
                        println!("{}", response);
                    }
                    Err(e) => {
                        println!("Error processing request: {:?}", e);
                        if let Err(why) = msg.channel_id.say(&ctx.http, format!("Error processing request: {:?}", e)).await {
                            println!("Error sending error message: {:?}", why);
                        }
                    }
                }


                // match self.rig_agent.process_message(&content).await {
                //     Ok(response) => {
                //         if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                //             error!("Error sending message: {:?}", why);
                //         }
                //     }
                //     Err(e) => {
                //         error!("Error processing message: {:?}", e);
                //         if let Err(why) = msg
                //             .channel_id
                //             .say(&ctx.http, format!("Error processing message: {:?}", e))
                //             .await
                //         {
                //             error!("Error sending error message: {:?}", why);
                //         }
                //     }
                // }
            } else {
                error!("Bot user ID not found in TypeMap");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        {
            let mut data = ctx.data.write().await;
            data.insert::<BotUserId>(ready.user.id);
        }

        let commands = Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command.name("hello").description("Say hello to the bot")
                })
                .create_application_command(|command| {
                    command
                        .name("ask")
                        .description("Ask the bot a question")
                        .create_option(|option| {
                            option
                                .name("query")
                                .description("Your question for the bot")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
        })
        .await;

        println!("Created the following global commands: {:#?}", commands);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");

    let rig_agent = Arc::new(RigAgent::new().await?);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            rig_agent: Arc::clone(&rig_agent),
        })
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
