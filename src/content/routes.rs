use poem::{Error, Result, web::Data};
use poem_openapi::{
    OpenApi,
    param::{Path, Query},
    payload::PlainText,
};
use reqwest::StatusCode;

use crate::data::Feed2PodcastURLs;

pub struct Router;

#[OpenApi(prefix_path = "content")]
impl Router {
    #[oai(path = "/:voice", method = "get")]
    async fn get_podcast_audio(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,
        Path(voice): Path<String>,
        Query(url): Query<String>,
        Query(selector): Query<String>,
        Query(ignore): Query<Vec<String>>,
    ) -> Result<PlainText<String>> {
        let content = reqwest::get(url.clone())
            .await
            .map_err(|e| {
                Error::from_string(
                    format!("Unable to fetch article content: {e}"),
                    StatusCode::BAD_REQUEST,
                )
            })?
            .text()
            .await
            .map_err(|_| Error::from_string("Invalid article content!", StatusCode::BAD_REQUEST))?;

        let doc = scraper::Html::parse_document(&content);
        let body_selector = scraper::Selector::parse(selector.as_str()).unwrap();

        let mut text_content = String::new();

        // Extract and append main article body
        for body in doc.select(&body_selector) {
            let paragraphs = body.text().collect::<String>();
            if !paragraphs.trim().is_empty() {
                text_content.push_str(&format!("{}\n", paragraphs));
            }
        }

        // Remove ignored elements content
        for ignore_tag in &ignore {
            let ignore_selector = scraper::Selector::parse(ignore_tag.as_str()).unwrap();
            for element_to_ignore in doc.select(&ignore_selector) {
                if let Some(text) = element_to_ignore.text().next() {
                    text_content = text_content.replace(text, "");
                }
            }
        }

        Ok(PlainText(text_content))
    }
}
