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
    #[oai(path = "/:voice", method = "get")]
    async fn podcast_feed(
        &self,
        Data(app_urls): Data<&Feed2PodcastURLs>,
        Path(voice): Path<String>,
        Query(url): Query<String>,
        Query(selector): Query<String>,
    ) -> Result<PlainText<String>> {
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

                    let mut enclosure = Enclosure::default();
                    enclosure.set_url(
                        Url::parse_with_params(
                            &format!("{}/{}", app_urls.base, voice),
                            &[("url", url.clone()), ("selector", selector.clone())],
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
}
