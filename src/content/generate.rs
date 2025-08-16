use std::path::Path;

use poem::{Error, Result};
use reqwest::StatusCode;
use rss::Channel;
use serde_json::json;
use tokio::sync::Semaphore;

pub async fn generate_podcast(
    file_path: &Path,
    feed_url: &str,
    entry_uid: &str,
    voice: &str,
    ignore: &mut Vec<String>,
    normalize: bool,
    tts_api_base: &str,
    tts_model: &str,
    permit: &Semaphore,
) -> Result<Vec<u8>> {
    let _perm = permit.acquire().await.map_err(|e| {
        Error::from_string(
            format!("Failed to acquire permit for podcast generation: {e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    Ok(if !file_path.exists() {
        let content = reqwest::get(feed_url)
            .await
            .map_err(|e| {
                Error::from_string(
                    format!("Unable to fetch original feed: {e}"),
                    StatusCode::BAD_REQUEST,
                )
            })?
            .bytes()
            .await
            .map_err(|_| Error::from_string("Invalid feed content!", StatusCode::BAD_REQUEST))?;

        let channel = Channel::read_from(&content[..]).map_err(|e| {
            Error::from_string(
                format!("Unable to parse feed: {e}"),
                StatusCode::BAD_REQUEST,
            )
        })?;

        let item = channel
            .items
            .into_iter()
            .find(|el| match &el.guid {
                Some(item_guid) => item_guid.value == *entry_uid,
                None => false,
            })
            .ok_or(Error::from_string(
                format!("Item with GUI '{entry_uid}' does not exist!"),
                StatusCode::BAD_REQUEST,
            ))?;

        let mut text_content: String;

        // Required to fix: rustc: future is not `Send` as this value is used across an await
        {
            let article = item.description.ok_or(Error::from_string(
                "No content found!",
                StatusCode::NOT_FOUND,
            ))?;

            let doc = scraper::Html::parse_document(&article);

            // Extract and append main article body
            text_content = doc.root_element().text().collect();

            // Remove ignored elements content
            ignore.extend([String::from("style"), String::from("script")]);
            for ignore_tag in ignore {
                let ignore_selector =
                    scraper::Selector::parse(ignore_tag.as_str()).map_err(|e| {
                        Error::from_string(
                            format!("Invalid selector '{}': {}", ignore_tag, e),
                            StatusCode::BAD_REQUEST,
                        )
                    })?;

                for element_to_ignore in doc.select(&ignore_selector) {
                    let text_to_ignore = element_to_ignore.text().collect::<String>();

                    text_content = text_content.replace(text_to_ignore.trim(), "");
                }
            }
        }

        println!("{}", &text_content);

        let client = reqwest::Client::new();
        let tts_req_body = json!({ "input": text_content, "model": tts_model, "voice": voice, "normalization_options": { "normalize": normalize}});
        let podcast = client
            .post(format!("{}/audio/speech", tts_api_base))
            .body(tts_req_body.to_string())
            .send()
            .await
            .map_err(|e| {
                Error::from_string(
                    format!("Unable to get response from TTS Server: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
            .error_for_status()
            .map_err(|e| {
                Error::from_string(
                    format!("Unable to get response from TTS Server: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
            .bytes()
            .await
            .map_err(|e| {
                Error::from_string(
                    format!("Failed to read TTS response body: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        std::fs::write(file_path, &podcast).map_err(|e| {
            Error::from_string(
                format!("Failed to write audio file: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        podcast.into()
    } else {
        std::fs::read(file_path).map_err(|e| {
            Error::from_string(
                format!("Failed to read audio file: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?
    })
}
