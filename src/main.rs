use std::collections::HashSet;
use std::env;

use color_eyre::eyre::Result;
use ddg::RelatedTopic;
use dotenv::dotenv;
use tracing::info;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    help_commands,
    macros::{command, group, help, hook},
    Args, CommandGroup, CommandResult, HelpOptions, StandardFramework,
};
use serenity::model::{
    channel::Message,
    gateway::Ready,
    prelude::{Activity, UserId},
};
use serenity::utils::Colour;

pub mod search;
use search::search;

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
#[help]
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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;
    tracing_subscriber::fmt::init();
    // Setting up basic structures
    color_eyre::install()?;

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

    // Start the bot
    client.start().await?;
    Ok(())
}

/// Ping command
#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(ctx, "Pong!").await?;
    Ok(())
}

/// Query DDG for search
#[command]
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
                    e.title("Search result:");
                    let search_result = search_result.related_topics;
                    for topic in search_result {
                        match topic {
                            RelatedTopic::TopicResult(topic_res) => {
                                e.field("URL", topic_res.first_url, false);
                                e.field("Info", topic_res.text, false);
                            }
                            RelatedTopic::Topic(_t) => continue,
                        }
                    }
                }
                e.footer(|f| {
                    f.icon_url("https://duckduckgo.com/assets/icons/meta/DDG-iOS-icon_152x152.png");
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
