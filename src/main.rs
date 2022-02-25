use std::{collections::HashSet, env, sync::Arc};

use color_eyre::eyre::Result;
use ddg::{RelatedTopic, Response};
use dotenv::dotenv;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tracing::{error, info};

use serenity::async_trait;
use serenity::builder::CreateEmbed;
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
    interactions::{
        application_command::{
            ApplicationCommand, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
        Interaction, InteractionResponseType,
    },
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

// Slash command response type
enum SlashCommandResponse {
    Basic(String),
    Rich(CreateEmbed),
}

// We declare our commands here
#[group]
#[commands(ping, s)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Basic slash commands support
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "ping" => {
                    let mut mes = MessageBuilder::new();
                    mes.push_bold("Pong!");
                    SlashCommandResponse::Basic(mes.build())
                }
                "s" => {
                    let options = command
                        .data
                        .options
                        .get(0)
                        .unwrap()
                        .resolved
                        .as_ref()
                        .unwrap();
                    if let ApplicationCommandInteractionDataOptionValue::String(query) = options {
                        let embed = create_search_embed(query.to_string()).await.unwrap();
                        SlashCommandResponse::Rich(embed)
                    } else {
                        let mut mes = MessageBuilder::new();
                        mes.push_bold("Please provide a valid search!");
                        SlashCommandResponse::Basic(mes.build())
                    }
                }
                _ => SlashCommandResponse::Basic(String::from("Not implemented :(")),
            };
            if let Err(why) = command
                .create_interaction_response(ctx, |res| {
                    res.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|mes| match content {
                            SlashCommandResponse::Basic(c) => mes.content(c),
                            SlashCommandResponse::Rich(e) => mes.add_embed(e),
                        })
                })
                .await
            {
                error!("Cannot respond to slash command: {}", why);
            }
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} successfully connected!", ready.user.name);
        ctx.set_activity(Activity::playing("searching the web! | ;help"))
            .await;
        let commands = ApplicationCommand::set_global_application_commands(ctx, |cmds| {
            cmds.create_application_command(|command| {
                command.name("ping").description("The ping command")
            })
            .create_application_command(|command| {
                command
                    .name("s")
                    .description("Search DuckDuckGo for result")
                    .create_option(|opt| {
                        opt.name("query")
                            .description("The thing you want to search")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
            })
        })
        .await;
        let commands_list = commands
            .unwrap()
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<String>>();
        info!(
            "The bot has the following global slash commands: {:?}",
            commands_list
        );
    }
    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("SBot successfully reconnected!");
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
    color_eyre::install()?;
    dotenv().ok();
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt().init();

    // The bot config
    let token = env::var("DISCORD_TOKEN")?;
    let app_id = env::var("APP_ID")?.parse::<u64>()?;
    let framework = StandardFramework::new()
        .configure(|c| c.prefix(";"))
        .before(before)
        .group(&GENERAL_GROUP)
        .help(&SBOT_HELP);
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .application_id(app_id)
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
#[only_in(guilds)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    // We return the latency when using this command
    let data = ctx.data.read().await;

    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            error!("There was a problem getting the shard manager");
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
            error!("No shard found");
            msg.reply(ctx, "No shard found").await?;

            return Ok(());
        }
    };

    // We only return the latency when it exists
    let mut message = MessageBuilder::new();
    match runner.latency {
        Some(latency) => message
            .push_bold("Pong!")
            .push(format!(" ({}ms)", latency.as_millis())),

        None => message.push_bold("Pong!"),
    };
    msg.channel_id.say(ctx, message.build()).await?;
    Ok(())
}

/// Query DuckDuckGo for search results
#[command]
#[only_in(guilds)]
#[usage = "<query>"]
#[example = "discord"]
async fn s(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    let query = arg.rest();
    let embed = create_search_embed(query.to_string()).await?;
    msg.channel_id
        .send_message(ctx, |m| {
            m.set_embed(embed);
            m
        })
        .await?;
    Ok(())
}

async fn create_search_embed(query: String) -> Result<CreateEmbed> {
    let (tx, mut rx) = oneshot::channel::<Response>();
    tokio::spawn(async move {
        let query = search(&query).await.unwrap();
        tx.send(query).unwrap();
    })
    .await?;
    let search_result = rx.try_recv()?;
    let mut e = CreateEmbed::default();
    e.colour(Colour::ORANGE);
    if search_result.abstract_text != "" {
        let mut title = MessageBuilder::new();
        title.push_bold(search_result.heading);
        e.title(title.build());

        e.description(search_result.abstract_text);
        e.url(search_result.abstract_url);
        if search_result.image != "" {
            e.image(format!("https://duckduckgo.com/{}", search_result.image));
        }
    } else {
        if search_result.related_topics.len() != 0 {
            let mut title = MessageBuilder::new();
            title.push_bold("Search results:");
            e.title(title.build());

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
        f.text("Results from DuckDuckGo");
        f
    });
    Ok(e)
}
