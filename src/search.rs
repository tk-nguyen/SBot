use color_eyre::eyre::Result;
use ddg::{Query, Response};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use tracing::info;

const INVIDIOUS_URL: &'static str = "https://youtube.076.ne.jp";
const BOT_NAME: &'static str = "sbot_discordbot";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Video {
    #[serde(rename = "type")]
    pub result_type: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
}

pub fn search(term: &str) -> Result<Response> {
    let query = Query::new(term, BOT_NAME).skip_disambig();
    let res = query.execute()?;
    Ok(res)
}

pub async fn youtube_search(query: &str) -> Result<Video> {
    info!("Got the query: {}", query);

    let request = Client::new()
        .request(
            Method::GET,
            format!(
                "{base}{path}",
                base = INVIDIOUS_URL,
                path = "/api/v1/search"
            ),
        )
        .query(&[("q", query)]);

    let result = request.send().await?.json::<Vec<Video>>().await?;

    match result.len() {
        0 => Ok(Video {
            result_type: None,
            title: None,
            video_id: None,
        }),
        _ => Ok(result.into_iter().next().unwrap()),
    }
}
