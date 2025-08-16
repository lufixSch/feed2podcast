use std::{fs::create_dir_all, path, sync::Arc};

use eyre::eyre;
use poem::{Error, Result, web::Data};
use poem_openapi::{
    OpenApi,
    param::{Path, Query},
    payload::Binary,
};
use reqwest::StatusCode;
use tokio::sync::Semaphore;
use url::Url;

use crate::{
    content::generate::generate_podcast,
    data::{Feed2PodcastDirs, Feed2PodcastTTSConfig, Feed2PodcastURLs},
    schemas::{DownloadFileResponse, CategoryTags},
};

pub struct Router;

/// Convert URL string to posix path
fn url_to_path(url: &str) -> eyre::Result<String> {
    match Url::parse(url) {
        Ok(url) => {
            Ok(String::from(url.host_str().ok_or(eyre!("Got URL without howt!"))?) + url.path())
        }
        Err(_) => Ok(String::from(url)),
    }
}

#[OpenApi(prefix_path = "content", tag = "CategoryTags::Feed")]
impl Router {
    /// Create Podcast audio (on demand) for a given article in a RSS Feed
    /// Caches audio to reduce response time for recurring requests.
    #[oai(path = "/:voice", method = "get")]
    async fn get_podcast_audio(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,
        Data(app_dirs): Data<&Feed2PodcastDirs>,
        Data(tts_conf): Data<&Feed2PodcastTTSConfig>,
        Data(permit): Data<&Arc<Semaphore>>,

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

        let audio = generate_podcast(
            &audio_path,
            &url,
            &uid,
            &voice,
            &mut ignore,
            normalize,
            &app_urls.tts,
            &tts_conf.model,
            permit,
        )
        .await?;

        Ok(DownloadFileResponse::Audio(
            Binary(audio),
            String::from("audio/mpeg"),
        ))
    }
}
