use color_eyre::eyre::Result;
use ddg::{Query, Response};

const BOT_NAME: &'static str = "sbot_discordbot";
pub async fn search(term: &str) -> Result<Response> {
    let query = Query::new(term, BOT_NAME).skip_disambig();
    let res = query.execute()?;
    Ok(res)
}
