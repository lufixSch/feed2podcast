use std::{fs::create_dir_all, path};

use eyre::eyre;
use poem::{Error, Result, web::Data};
use poem_openapi::{
    ApiResponse, OpenApi,
    param::{Path, Query},
    payload::Binary,
};
use reqwest::StatusCode;
use rss::Channel;
use serde_json::json;
use url::Url;

use crate::data::{Feed2PodcastDirs, Feed2PodcastURLs};

pub struct Router;

#[derive(Debug, ApiResponse)]
enum DownloadFileResponse {
    #[oai(status = 200)]
    Audio(Binary<Vec<u8>>, #[oai(header = "content-type")] String),
}

/// Convert URL string to posix path
fn url_to_path(url: &str) -> eyre::Result<String> {
    match Url::parse(url) {
        Ok(url) => {
            Ok(String::from(url.host_str().ok_or(eyre!("Got URL without howt!"))?) + url.path())
        }
        Err(_) => Ok(String::from(url)),
    }
}

#[OpenApi(prefix_path = "content")]
impl Router {
    /// Create Podcast audio (on demand) for a given article in a RSS Feed
    /// Caches audio to reduce response time for recurring requests.
    #[oai(path = "/:voice", method = "get")]
    async fn get_podcast_audio(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,
        Data(app_dirs): Data<&Feed2PodcastDirs>,

        /// The voice to use for the podcast
        Path(voice): Path<String>,

        /// The Feed URL
        Query(url): Query<String>,

        /// The GUID of the article
        Query(uid): Query<String>,

        /// HTML elements/CSS Selectors to ignore when parsing the content
        Query(mut ignore): Query<Vec<String>>,

        /// Whether to normalize text for TTS (will improve TTS but can lead to errors when content
        /// includes long numbers)
        Query(normalize): Query<bool>,
    ) -> Result<DownloadFileResponse> {
        let url_path = url_to_path(&url).map_err(|e| {
            Error::from_string(
                format!("Unable to create cache directory from feed URL: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let id_path = url_to_path(&uid).map_err(|e| {
            Error::from_string(
                format!("Unable to create cache directory from UID: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let file_dir = path::Path::new(&app_dirs.cache)
            .join(&url_path)
            .join(&id_path);
        let audio_path = file_dir.join(format!("{voice}.mp3"));

        if !file_dir.exists() {
            create_dir_all(file_dir).map_err(|_| {
                Error::from_string(
                    "Unable to create cache directory for podcast",
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        };

        let audio = if !audio_path.exists() {
            let content = reqwest::get(url.clone())
                .await
                .map_err(|e| {
                    Error::from_string(
                        format!("Unable to fetch original feed: {e}"),
                        StatusCode::BAD_REQUEST,
                    )
                })?
                .bytes()
                .await
                .map_err(|_| {
                    Error::from_string("Invalid feed content!", StatusCode::BAD_REQUEST)
                })?;

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
                    Some(item_guid) => item_guid.value == uid,
                    None => false,
                })
                .ok_or(Error::from_string(
                    format!("Item with GUI '{uid}' does not exist!"),
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
            let tts_req_body = json!({ "input": text_content, "voice": voice, "normalization_options": { "normalize": normalize}});
            let podcast = client
                .post(format!("{}/audio/speech", app_urls.tts))
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

            std::fs::write(audio_path, &podcast).map_err(|e| {
                Error::from_string(
                    format!("Failed to write audio file: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

            podcast.into()
        } else {
            std::fs::read(audio_path).map_err(|e| {
                Error::from_string(
                    format!("Failed to read audio file: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
        };

        Ok(DownloadFileResponse::Audio(
            Binary(audio),
            String::from("audio/mpeg"),
        ))
    }
}
