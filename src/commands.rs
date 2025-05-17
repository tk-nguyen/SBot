use crate::{Context, DUCKDUCKGO_ICON, search::*};
use color_eyre::{Result, eyre::eyre};
use ddg::Response;
use poise::{
    CreateReply,
    samples::HelpConfiguration,
    serenity_prelude::{Colour, CreateEmbed, CreateEmbedFooter},
};
use tokio::sync::{mpsc, oneshot};

/// Show bot help
#[poise::command(slash_command, prefix_command, track_edits)]
pub(crate) async fn help(
    ctx: Context<'_>,
    #[description = "Command to get help for"]
    #[rest]
    mut command: Option<String>,
) -> Result<()> {
    if ctx.invoked_command_name() != "help" {
        command = match command {
            Some(c) => Some(format!("{} {}", ctx.invoked_command_name(), c)),
            None => Some(ctx.invoked_command_name().to_string()),
        };
    }
    let extra_text_at_bottom = "A simple search bot for Discord, using DuckDuckGo.\n
                                              To get help with an individual command, pass its name as an argument to this command.";

    let config = HelpConfiguration {
        show_subcommands: true,
        ephemeral: true,
        extra_text_at_bottom,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}
/// Return the ping from the bot to Discord
#[poise::command(slash_command, prefix_command)]
pub(crate) async fn ping(ctx: Context<'_>) -> Result<()> {
    let ping = match ctx.ping().await.as_millis() {
        0 => "Just connected to Discord, no ping yet!".to_string(),
        p => format!("**{p} ms**",),
    };
    ctx.say(format!("Pong! {}", ping)).await?;
    Ok(())
}

/// Query DuckDuckGo for search results
#[poise::command(slash_command, prefix_command, track_edits)]
pub(crate) async fn s(
    ctx: Context<'_>,
    #[description = "Search query"]
    #[rest]
    query: String,
) -> Result<()> {
    let embed = create_search_embed(query).await?;
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

async fn create_search_embed(query: String) -> Result<CreateEmbed> {
    let (tx, rx) = oneshot::channel::<Response>();
    let first_query = query.clone();
    tokio::spawn(async move { search(&first_query, tx).await });
    let search_result = rx.await?;

    // If the abstract has content, build the embed from it
    let e = match (
        search_result.abstract_text.is_empty(),
        search_result.image.is_empty(),
    ) {
        (false, true) => CreateEmbed::default()
            .colour(Colour::ORANGE)
            .title(format!("**{}**", search_result.heading))
            .description(search_result.abstract_text)
            .url(search_result.abstract_url)
            .footer(
                CreateEmbedFooter::new("")
                    .icon_url(DUCKDUCKGO_ICON)
                    .text("Results from DuckDuckGo"),
            ),
        (false, false) => CreateEmbed::default()
            .colour(Colour::ORANGE)
            .title(format!("**{}**", search_result.heading))
            .description(search_result.abstract_text)
            .url(search_result.abstract_url)
            .image(format!("https://duckduckgo.com/{}", search_result.image))
            .footer(
                CreateEmbedFooter::new("")
                    .icon_url(DUCKDUCKGO_ICON)
                    .text("Results from DuckDuckGo"),
            ),
        (true, _) => {
            let (scrape_tx, mut scrape_rx) = mpsc::unbounded_channel::<ScrapeResponse>();
            tokio::spawn(async move { search_scrape(query, scrape_tx).await });
            let result = scrape_rx
                .recv()
                .await
                .ok_or(eyre!("Cannot receive scrape result!"))?;
            CreateEmbed::default()
                .colour(Colour::ORANGE)
                .title(format!("**{}**", result.title))
                .url(result.url)
                .description(result.content)
                .footer(
                    CreateEmbedFooter::new("")
                        .icon_url(DUCKDUCKGO_ICON)
                        .text("Results from DuckDuckGo"),
                )
        }
    };
    Ok(e)
}
