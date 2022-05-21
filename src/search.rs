use color_eyre::eyre::Result;
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

const BOT_NAME: &'static str = "sbot_discordbot";
const DUCKDUCKGO_URL: &'static str = "https://duckduckgo.com/html/";
pub async fn search(term: &str) -> Result<Response> {
    let query = Query::new(term, BOT_NAME).skip_disambig();
    let res = query.execute()?;
    Ok(res)
}

// We scrape the duckduckgo html search result if the Instant Answer API return nothing
pub async fn search_scrape(term: &str) -> Result<ScrapeResponse> {
    let search = Client::builder()
        .user_agent(BOT_NAME)
        .build()?
        .get(DUCKDUCKGO_URL)
        .query(&[("q", term)])
        .send()
        .await?;
    let document = Html::parse_fragment(&search.text().await?);
    let selector = Selector::parse(r#"div[class="links_main links_deep result__body"]"#).unwrap();

    let result = document.select(&selector).next().unwrap();

    let header = result
        .select(&Selector::parse(r#"a[class="result__a"]"#).unwrap())
        .next()
        .unwrap();
    let href = header.value().attr("href").unwrap();
    let title = header.text().next().unwrap();
    let href_absolute = Url::parse(&format!("{}:{}", "https", href))?;
    let query = href_absolute
        .query_pairs()
        .filter(|(n, _)| n == "uddg")
        .map(|(_, v)| v)
        .next()
        .unwrap();
    let url = percent_encoding::percent_decode(query.as_bytes()).decode_utf8_lossy();

    let content = result
        .select(&Selector::parse(r#"a[class="result__snippet"]"#).unwrap())
        .next()
        .unwrap()
        .text()
        .collect::<Vec<_>>()
        .join("");

    Ok(ScrapeResponse::new(
        title.to_string(),
        url.to_string(),
        content,
    ))
}
