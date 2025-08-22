use std::path;

use poem::{Error, Result, error::InternalServerError, web::Data};
use poem_openapi::{
    OpenApi,
    param::Path,
    payload::{Binary, Json},
};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;

use crate::{
    cache,
    data::{Feed2PodcastDirs, Feed2PodcastTTSConfig, Feed2PodcastURLs},
    schemas::{CategoryTags, DownloadFileResponse},
};

#[derive(Deserialize)]
struct AvailableVoices {
    voices: Vec<String>,
}

pub async fn generate_demo(
    file_path: &path::Path,
    voice: &str,
    tts_api_base: &str,
    tts_model: &str,
) -> Result<Vec<u8>> {
    Ok(if !file_path.exists() {
        let client = reqwest::Client::new();
        let tts_req_body = json!({ "input": "The quick brown fox jumps over the lazy dog.", "model": tts_model, "voice": voice });
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

pub struct Router;

#[OpenApi(prefix_path = "demo", tag = "CategoryTags::Demo")]
impl Router {
    #[oai(path = "/", method = "get")]
    async fn get_available_demos(
        &self,
        Data(tts_conf): Data<&Feed2PodcastTTSConfig>,
        Data(app_urls): Data<&Feed2PodcastURLs>,
    ) -> Result<Json<Vec<String>>> {
        let available_voices = match &tts_conf.voices {
            Some(v) => v.clone(),
            None => {
                reqwest::get(&format!("{}/audio/voices", app_urls.tts))
                    .await
                    .map_err(InternalServerError)?
                    .error_for_status()
                    .map_err(InternalServerError)?
                    .json::<AvailableVoices>()
                    .await
                    .map_err(InternalServerError)?
                    .voices
            }
        };

        Ok(Json(available_voices))
    }

    #[oai(path = "/:voice", method = "get")]
    async fn get_demo_for_voice(
        &self,
        Data(tts_conf): Data<&Feed2PodcastTTSConfig>,
        Data(app_urls): Data<&Feed2PodcastURLs>,
        Data(app_dirs): Data<&Feed2PodcastDirs>,

        /// The voice to use for the demo
        Path(voice): Path<String>,
    ) -> Result<DownloadFileResponse> {
        let audio_path = cache::get_demo_path(&app_dirs.cache, &tts_conf.model, &voice)?;

        let audio = generate_demo(&audio_path, &voice, &app_urls.tts, &tts_conf.model).await?;

        Ok(DownloadFileResponse::Audio(
            Binary(audio),
            String::from("audio/mpeg"),
        ))
    }
}
