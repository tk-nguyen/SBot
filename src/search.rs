use color_eyre::eyre::{Context, Result, eyre};
use ddg::{Query, Response};
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct ScrapeResponse {
    pub title: String,
    pub url: String,
    pub content: String,
}

impl ScrapeResponse {
    pub fn new(title: String, url: String, content: String) -> Self {
        Self {
            title,
            url,
            content,
        }
    }
}

impl Default for ScrapeResponse {
    fn default() -> Self {
        Self {
            title: "No result found!".to_string(),
            url: Default::default(),
            content: Default::default(),
        }
    }
}

const BOT_NAME: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36";
const DUCKDUCKGO_URL: &str = "https://html.duckduckgo.com/html/";
// Normal search, using DuckDuckGo instant search API
pub async fn search(term: &str, tx: oneshot::Sender<Response>) -> Result<()> {
    let query = Query::new(term, BOT_NAME).skip_disambig();
    let res = query.execute()?;
    tx.send(res).unwrap();
    Ok(())
}

// We scrape the duckduckgo html search result if the Instant Answer API return nothing
pub async fn search_scrape(term: String, tx: mpsc::UnboundedSender<ScrapeResponse>) -> Result<()> {
    // Initial request to search
    let search = Client::builder()
        .user_agent(BOT_NAME)
        .build()?
        .get(DUCKDUCKGO_URL)
        .query(&[("q", term.as_str())])
        .send()
        .await?;
    let document = Html::parse_fragment(&search.text().await?);
    // The search result elements
    let result = document
        .select(&Selector::parse(r#"div[class="links_main links_deep result__body"]"#).unwrap())
        .next()
        .ok_or_else(|| {
            tx.clone().send(ScrapeResponse::default()).unwrap();
            eyre!("Cannot find the search result elements!")
        })?;

    // The header, which contain the final url and title
    let header = result
        .select(&Selector::parse(r#"a[class="result__a"]"#).unwrap())
        .next()
        .ok_or_else(|| {
            tx.clone().send(ScrapeResponse::default()).unwrap();
            eyre!("Cannot find the search result link!")
        })?;
    let url = header.value().attr("href").ok_or_else(|| {
        tx.clone().send(ScrapeResponse::default()).unwrap();
        eyre!("Cannot find the search result link!")
    })?;
    let title = header.text().next().ok_or_else(|| {
        tx.clone().send(ScrapeResponse::default()).unwrap();
        eyre!("Cannot find the search result title!")
    })?;

    // The search result content
    let content = result
        .select(&Selector::parse(r#"a[class="result__snippet"]"#).unwrap())
        .next()
        .ok_or_else(|| {
            tx.clone().send(ScrapeResponse::default()).unwrap();
            eyre!("Cannot find the search result content!")
        })?
        .text()
        .collect::<Vec<_>>()
        .join("");

    tx.send(ScrapeResponse::new(
        title.to_string(),
        url.to_string(),
        content,
    ))
    .wrap_err("Cannot send the search result to the main thread!")
}
