use askama::Template;
use poem::Result;
use poem::error::InternalServerError;
use poem::web::Data;
use poem_openapi::{OpenApi, payload::Html};
use serde::Deserialize;

use crate::data::{Feed2PodcastTTSConfig, Feed2PodcastURLs};
use crate::schemas::CategoryTags;
use crate::webui::templates;

pub struct Router;

#[derive(Deserialize)]
struct AvailableVoices {
    voices: Vec<String>,
}

async fn get_voices(
    tts_conf: &Feed2PodcastTTSConfig,
    app_urls: &Feed2PodcastURLs,
) -> Result<Vec<String>> {
    Ok(match &tts_conf.voices {
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
    })
}

#[OpenApi(tag = "CategoryTags::WebUI")]
impl Router {
    #[oai(path = "/", method = "get")]
    async fn create_feed(
        &self,

        Data(tts_conf): Data<&Feed2PodcastTTSConfig>,
        Data(app_urls): Data<&Feed2PodcastURLs>,
    ) -> Result<Html<String>> {
        let available_voices = get_voices(tts_conf, app_urls).await?;

        Ok(Html(
            templates::Index {
                title: "Feed2Podcast WebUI",
                description: "Convert RSS feeds to podcasts with TTS.",
                voices: available_voices,
                api_base: app_urls.base.clone()
            }
            .render()
            .map_err(InternalServerError)?,
        ))
    }

    #[oai(path = "/demo", method = "get")]
    async fn voice_demo(
        &self,

        Data(tts_conf): Data<&Feed2PodcastTTSConfig>,
        Data(app_urls): Data<&Feed2PodcastURLs>,
    ) -> Result<Html<String>> {
        let available_voices = get_voices(tts_conf, app_urls).await?;

        Ok(Html(
            templates::Demo {
                title: "Feed2Podcast Voice Demo",
                description: "Demonstrate available TTS Voices.",
                voices: available_voices,
            }
            .render()
            .map_err(InternalServerError)?,
        ))
    }
}
