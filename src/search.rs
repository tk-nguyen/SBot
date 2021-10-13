use color_eyre::eyre::Result;
use ddg::{Query, Response};
use tracing::info;

const BOT_NAME: &'static str = "sbot_discordbot";

pub fn ddg_search(term: &str) -> Result<Response> {
    let query = Query::new(term, BOT_NAME).skip_disambig();
    let res = query.execute()?;
    Ok(res)
}
