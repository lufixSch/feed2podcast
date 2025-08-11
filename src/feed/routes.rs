use poem::{Error, Result, web::Data};
use poem_openapi::{
    OpenApi,
    param::{Path, Query},
    payload::PlainText,
};
use reqwest::StatusCode;
use rss::{Channel, Enclosure, Item};
use url::Url;

use crate::data::Feed2PodcastURLs;

pub struct Router;

#[OpenApi(prefix_path = "feed")]
impl Router {
    /// Generate a podcast feed from a regular RSS feed where the link to the audio points to the
    /// "Get Podcast Audio" endpoint
    #[oai(path = "/:voice", method = "get")]
    async fn podcast_feed(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,

        /// The voice to use for the podcast
        Path(voice): Path<String>,
        /// The Feed URL
        Query(url): Query<String>,
        /// HTML elements/CSS Selectors to ignore when parsing the content
        Query(ignore): Query<Option<Vec<String>>>,

        /// Whether to normalize text for TTS (will improve TTS but can lead to errors when content
        /// includes long numbers)
        Query(normalize): Query<bool>,
    ) -> Result<PlainText<String>> {
        let ingore_unpacked = ignore.unwrap_or_default();

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
            .map_err(|_| Error::from_string("Invalid feed content!", StatusCode::BAD_REQUEST))?;

        let channel = Channel::read_from(&content[..]).map_err(|e| {
            Error::from_string(
                format!("Unable to parse feed: {e}"),
                StatusCode::BAD_REQUEST,
            )
        })?;

        let mut podcast_ch = channel.clone();

        podcast_ch.set_items(
            channel
                .items()
                .iter()
                .map(|item| {
                    let mut new_item = item.clone();

                    let uid = item
                        .guid
                        .clone()
                        .ok_or(Error::from_string(
                            "Unable to get GUID for some items",
                            StatusCode::BAD_REQUEST,
                        ))?
                        .value;

                    let mut enclosure = Enclosure::default();
                    let mut url_params: Vec<(&str, String)> = ingore_unpacked
                        .clone()
                        .into_iter()
                        .map(|i| ("ignore", i))
                        .collect();
                    url_params.extend([
                        ("url", url.clone()),
                        ("uid", uid),
                        (
                            "normalize",
                            String::from(if normalize { "true" } else { "false" }),
                        ),
                    ]);

                    enclosure.set_url(
                        Url::parse_with_params(
                            &format!("{}/api/content/{}", app_urls.base, voice),
                            &url_params,
                        )
                        .map_err(|e| {
                            Error::from_string(
                                format!("Unable to generate Content url for {}: {}", url, e),
                                StatusCode::BAD_REQUEST,
                            )
                        })?
                        .as_str(),
                    );
                    enclosure.set_mime_type("audio/mpeg");

                    new_item.set_enclosure(enclosure);
                    Ok(new_item)
                })
                .collect::<Result<Vec<Item>>>()?,
        );

        Ok(PlainText(podcast_ch.to_string()))
    }

    /// Helper endpoint to generate feed URL directly from the API docs
    #[oai(path = "/build/:voice", method = "get")]
    async fn build_feed_url(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,

        /// The voice to use for the podcast
        Path(voice): Path<String>,
        /// The Feed URL
        Query(url): Query<String>,
        /// HTML elements/CSS Selectors to ignore when parsing the content
        Query(ignore): Query<Option<Vec<String>>>,

        /// Whether to normalize text for TTS (will improve TTS but can lead to errors when content
        /// includes long numbers). Defaults to `true`
        Query(normalize): Query<Option<bool>>,
    ) -> Result<PlainText<String>> {
        let ingore_unpacked = ignore.unwrap_or_default();
        let normalize_unpacked = normalize.unwrap_or(true);

        let mut url_params: Vec<(&str, String)> =
            ingore_unpacked.into_iter().map(|i| ("ignore", i)).collect();
        url_params.extend([
            ("url", url.clone()),
            (
                "normalize",
                String::from(if normalize_unpacked { "true" } else { "false" }),
            ),
        ]);

        let feed_url = Url::parse_with_params(
            &format!("{}/api/feed/{}", app_urls.base, voice),
            &url_params,
        )
        .map_err(|e| {
            Error::from_string(
                format!("Unable to generate Content url for {}: {}", url, e),
                StatusCode::BAD_REQUEST,
            )
        })?;

        Ok(PlainText(feed_url.to_string()))
    }
}
