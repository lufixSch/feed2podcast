use std::sync::Arc;

use poem::{Result, web::Data};
use poem_openapi::{
    OpenApi,
    param::{Path, Query},
    payload::Binary,
};
use tokio::sync::Semaphore;

use crate::{
    cache,
    content::generate::generate_podcast,
    data::{Feed2PodcastDirs, Feed2PodcastTTSConfig, Feed2PodcastURLs},
    schemas::{CategoryTags, DownloadFileResponse},
};

pub struct Router;

#[OpenApi(prefix_path = "content", tag = "CategoryTags::Feed")]
impl Router {
    /// Create Podcast audio (on demand) for a given article in a RSS Feed
    /// Caches audio to reduce response time for recurring requests.
    #[oai(path = "/:voice", method = "get")]
    async fn get_podcast_audio(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,
        Data(app_dirs): Data<&Feed2PodcastDirs>,
        Data(cache_cleanup): Data<&cache::CleanupMethod>,
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
        let audio_path = cache::get_podcast_path(&app_dirs.cache, &url, &uid, &voice)?;

        let (audio, was_generated) = generate_podcast(
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

        if was_generated {
            // Run cache cleanup in background
            tokio::spawn(cache::run_cleanup_task(
                app_dirs.cache.clone(),
                cache_cleanup.clone(),
            ));
        }

        Ok(DownloadFileResponse::Audio(
            Binary(audio),
            String::from("audio/mpeg"),
        ))
    }
}
