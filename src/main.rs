use std::env;

use color_eyre::eyre::{Error, Result};
use dotenvy::dotenv;
use poise::{
    Framework, FrameworkOptions, PrefixFrameworkOptions,
    builtins::register_globally,
    serenity_prelude::{ActivityData, ClientBuilder, GatewayIntents},
};

pub(crate) mod commands;
pub(crate) mod search;
use commands::*;

// This icon is used in the embed for attribution
const DUCKDUCKGO_ICON: &str = "https://duckduckgo.com/assets/icons/meta/DDG-iOS-icon_152x152.png";

struct Data {}

type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenv().ok();
    if env::var("RUST_LOG").is_err() {
        // Rust 2024 make env::set_var unsafe
        unsafe {
            env::set_var("RUST_LOG", "info");
        }
    }
    // Install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    // The bot config
    let token = env::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES;
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![help(), ping(), s()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from(";")),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                ctx.set_activity(Some(ActivityData::playing("searching the web! | ;help")));
                register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();
    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;
    // Start the bot
    client.start_autosharded().await?;
    Ok(())
}
