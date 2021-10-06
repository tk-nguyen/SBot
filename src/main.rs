use std::{collections::HashSet, env, sync::Arc};

use color_eyre::eyre::Result;
use ddg::RelatedTopic;
use dotenv::dotenv;
use tokio::sync::Mutex;
use tracing::{error, info};

use serenity::async_trait;
use serenity::client::{
    bridge::gateway::{ShardId, ShardManager},
    Client, Context, EventHandler,
};
use serenity::framework::standard::{
    help_commands,
    macros::{command, group, help, hook},
    Args, CommandGroup, CommandResult, HelpOptions, StandardFramework,
};
use serenity::model::{
    channel::Message,
    event::ResumedEvent,
    gateway::Ready,
    prelude::{Activity, UserId},
};
use serenity::prelude::TypeMapKey;
use serenity::utils::{Colour, MessageBuilder};

pub mod search;
use search::search;

// Shard management for latency measuring
struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

// We declare our commands here
#[group]
#[commands(ping, s)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} successfully connected!", ready.user.name);
        ctx.set_activity(Activity::playing("searching the web! | !help"))
            .await;
    }
    async fn resume(&self, _ctx: Context, _resume: ResumedEvent) {
        info!("Bot successfully reconnected!");
    }
}

// Logging requested command
#[hook]
#[instrument]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Got command '{}' by user '{}#{}'",
        command_name, msg.author.name, msg.author.discriminator
    );

    true
}

// A help command for the bot
#[help]
#[individual_command_tip = "A simple search bot for Discord, using DuckDuckGo\n\n\
To get help with an individual command, pass its name as an argument to this command."]
#[embed_success_colour(GOLD)]
#[embed_error_colour(RED)]
#[command_not_found_text = "Could not find: `{}`"]
#[strikethrough_commands_tip_in_dm = ""]
#[strikethrough_commands_tip_in_guild = ""]
async fn sbot_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

// This icon is used in the embed for attribution
const DUCKDUCKGO_ICON: &'static str =
    "https://duckduckgo.com/assets/icons/meta/DDG-iOS-icon_152x152.png";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    // Setting up basic structures

    // The bot config
    let token = env::var("DISCORD_TOKEN")?;
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .before(before)
        .group(&GENERAL_GROUP)
        .help(&SBOT_HELP);
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .await?;
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }
    // Start the bot
    client.start_autosharded().await?;
    Ok(())
}

/// Ping command, also return latency
#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    // We return the latency when using this command
    let data = ctx.data.read().await;

    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager")
                .await?;

            return Ok(());
        }
    };

    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    // Shards are backed by a "shard runner" responsible for processing events
    // over the shard, so we'll get the information about the shard runner for
    // the shard this command was sent over.
    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found").await?;

            return Ok(());
        }
    };
    let mut message = MessageBuilder::new();
    message.push_bold("Pong! ");
    if let Some(latency) = runner.latency {
        message.push(format!("({}ms)", latency.as_millis()));
    }
    msg.channel_id.say(ctx, message.build()).await?;
    Ok(())
}

/// Query DuckDuckGo for search results
#[command]
#[usage = "<query>"]
#[example = "discord"]
async fn s(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    info!(
        "'{}#{}' requested a search for '{}'",
        msg.author.name,
        msg.author.discriminator,
        arg.rest()
    );
    let search_result = search(arg.rest())?;

    msg.channel_id
        .send_message(ctx, |m| {
            m.add_embed(|e| {
                e.color(Colour::GOLD);
                if search_result.abstract_text != "" {
                    e.title(search_result.heading);
                    e.description(search_result.abstract_text);
                    e.url(search_result.abstract_url);
                    if search_result.image != "" {
                        e.image(format!("https://duckduckgo.com/{}", search_result.image));
                    }
                } else {
                    if search_result.related_topics.len() != 0 {
                        e.title("Search result:");
                        let search_result = search_result.related_topics;
                        for (idx, topic) in search_result.iter().enumerate() {
                            if let RelatedTopic::TopicResult(topic_res) = topic {
                                let mut res = MessageBuilder::new();
                                res.push(format!("{}\n", topic_res.first_url))
                                    .push(format!("{}\n", topic_res.text));
                                e.field(idx + 1, res.build(), true);
                            }
                        }
                    } else {
                        let mut res = MessageBuilder::new();
                        res.push_bold("No result found!");
                        e.description(res.build());
                    }
                }
                e.footer(|f| {
                    f.icon_url(DUCKDUCKGO_ICON);
                    f.text("Result from DuckDuckGo");
                    f
                });
                e
            });
            m
        })
        .await?;
    Ok(())
}
