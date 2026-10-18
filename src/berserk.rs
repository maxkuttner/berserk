
use scraper::{Html, Selector};
use reqwest::{Client, Response};
use url::Url;
use std::collections::HashSet;

pub async fn get_all_chapters() -> Result<Vec<String>, reqwest::Error>{
    let main_page = Url::parse("https://readberserk.com/").unwrap();
    let client = Client::new();

    let response = client
        .get(main_page.clone())
        .send()
        .await?;

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a[href]").unwrap();
    let mut chapters = HashSet::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            let url = main_page.join(href).unwrap();
            if url.domain() == main_page.domain() && url.path().starts_with("/chapter/"){
                chapters.insert(url.to_string());
            }
        }
    }
    let mut chapter_urls: Vec<_> = chapters.into_iter().collect();
    chapter_urls.sort();
    for c in &chapter_urls {
        println!("{}", c);
    }
    Ok(chapter_urls)
}

#[derive(Debug)]
pub struct Chapter {
    pub url: Url,
    pub pages: Vec<Url>,
}

pub async fn fetch_chapter(
    chapter_url: &str,
) -> Result<Chapter, reqwest::Error> {
    // Parse chapter URL
    let url = Url::parse(chapter_url).unwrap();

    let client = Client::new();

    // Fetch HTML
    let html = client
        .get(url.clone())
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // Parse HTML
    let document = Html::parse_document(&html);

    // Select page images
    let selector = Selector::parse("img[alt*='Page']").unwrap();

    let mut pages = Vec::new();

    for image in document.select(&selector) {
        let src = image
            .value()
            .attr("data-src")
            .or_else(|| image.value().attr("data-lazy-src"))
            .or_else(|| image.value().attr("src"));

        if let Some(src) = src {
            let page_url = url.join(src).unwrap();
            pages.push(page_url);
        }
    }

    Ok(Chapter { url, pages })
}