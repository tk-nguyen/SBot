use color_eyre::eyre::{eyre, Result};
use ddg::{Query, Response};
use reqwest::{Client, Url};
use scraper::{Html, Selector};

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

const BOT_NAME: &str = "sbot_discordbot";
const DUCKDUCKGO_URL: &str = "https://duckduckgo.com/html/";
pub async fn search(term: &str) -> Result<Response> {
    let query = Query::new(term, BOT_NAME).skip_disambig();
    let res = query.execute()?;
    Ok(res)
}

// We scrape the duckduckgo html search result if the Instant Answer API return nothing
pub async fn search_scrape(term: &str) -> Result<ScrapeResponse> {
    // Initial request to search
    let search = Client::builder()
        .user_agent(BOT_NAME)
        .build()?
        .get(DUCKDUCKGO_URL)
        .query(&[("q", term)])
        .send()
        .await?;
    let document = Html::parse_fragment(&search.text().await?);

    // The search result elements
    let result = document
        .select(&Selector::parse(r#"div[class="links_main links_deep result__body"]"#).unwrap())
        .next()
        .ok_or(eyre!("Cannot find the search result elements!"))?;

    // The header, which contain the final url and title
    let header = result
        .select(&Selector::parse(r#"a[class="result__a"]"#).unwrap())
        .next()
        .ok_or(eyre!("Cannot find the search result link!"))?;
    let href = header
        .value()
        .attr("href")
        .ok_or(eyre!("Cannot find the search result link!"))?;
    let title = header
        .text()
        .next()
        .ok_or(eyre!("Cannot find the header text!"))?;
    let href_absolute = Url::parse(&format!("{}:{}", "https", href))?;
    let query = href_absolute
        .query_pairs()
        .filter(|(n, _)| n == "uddg")
        .map(|(_, v)| v)
        .next()
        .ok_or(eyre!("Cannot find the final link from the result!"))?;
    let url = percent_encoding::percent_decode(query.as_bytes()).decode_utf8_lossy();

    // The search result content
    let content = result
        .select(&Selector::parse(r#"a[class="result__snippet"]"#).unwrap())
        .next()
        .ok_or(eyre!("Cannot find the search result's snippet!"))?
        .text()
        .collect::<Vec<_>>()
        .join("");

    Ok(ScrapeResponse::new(
        title.to_string(),
        url.to_string(),
        content,
    ))
}
