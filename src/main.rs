use std::{sync::Arc, time::Duration};

use poem::{
    EndpointExt, Route,
    listener::TcpListener,
    middleware::{Cors, Tracing},
};
use poem_openapi::OpenApiService;

use clap::Parser;
use eyre::{Result, eyre};

mod content;
mod demo;
mod feed;
mod webui;

mod cache;
mod data;
mod schemas;
use data::Feed2PodcastURLs;
use tokio::sync::Semaphore;
use tracing_subscriber::EnvFilter;

use crate::data::{Feed2PodcastDirs, Feed2PodcastTTSConfig};

#[derive(Parser, Debug)]
#[command(name = "feed2podcast")]
#[command(
    version,
    about = "Generate podcast feed from text based rss feeds using TTS"
)]
struct Args {
    /// URL to the API (make sure to change this when changing the port and using the API docs).
    #[arg(
        short,
        long,
        help = "URL to the API (make sure to change this when changing the port and using the API docs)",
        env = "FEED2PODCAST_URL",
        default_value_t = String::from("http://127.0.0.1:3000")
    )]
    url: String,

    /// The port on which the server listens for requests.
    #[arg(
        short,
        long,
        help = "The port on which the server listens for requests.",
        env = "FEED2PODCAST_PORT",
        default_value_t = 3000
    )]
    port: u16,

    /// Disable the SwaggerUI API docs (/docs).
    #[arg(
        short,
        long,
        help = "Disable the SwaggerUI API docs (/docs)",
        env = "FEED2PODCAST_DISABLE_DOCS",
        default_value_t = false
    )]
    disable_docs: bool,

    /// Cache directory for podcast files
    #[arg(
        short,
        long,
        help = "Cache directory for podcast files",
        env = "FEED2PODCAST_CACHE_DIR",
        default_value_t = String::from("./cache")
    )]
    cache_dir: String,

    /// URL to a OpenAI compatible TTS API.
    #[arg(
        short,
        long,
        help = "URL to a OpenAI compatible TTS API.
",
        env = "FEED2PODCAST_TTS_API",
        default_value = "http://127.0.0.1:5000/v1"
    )]
    tts_url: String,

    /// Available Voices for TTS (uses audio/voices if not set)
    #[arg(
        short,
        long,
        help = "Comma separated list of available Voices for TTS (uses audio/voices if not set)",
        env = "FEED2PODCAST_VOICES",
        value_delimiter = ','
    )]
    voices: Vec<String>,

    /// TTS Model to use
    #[arg(
        short,
        long,
        help = "TTS Model to use (Defaults to 'kokoro')",
        env = "FEED2PODCAST_MODEL",
        default_value = "kokoro"
    )]
    model: String,

    /// Max cache size
    #[arg(
        long,
        help = "Maximum cache size GB (Only one of --cache-size and --cache-age is taken into account, If both are provided --cache-size is used)",
        env = "FEED2PODCAST_MAX_CACHE_SIZE"
    )]
    cache_size: Option<u32>,

    /// Max cache age
    #[arg(
        long,
        help = "Maximum cache age in days (cleanup only runs when new podcast is generated)",
        env = "FEED2PODCAST_MAX_CACHE_AGE"
    )]
    cache_age: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let level = std::env::var("RUST_LOG").unwrap_or(String::from("trace"));
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(format!(
            "{}={level},poem={level}",
            env!("CARGO_PKG_NAME").replace("-", "_")
        )))
        .init();

    let args = Args::parse();

    tracing::info!("Starting server with config: {args:#?}");


    let cache_cleanup_method = if let Some(max_sz) = args.cache_size {
        cache::CleanupMethod::MaxStorage((max_sz as u64) * (1e9 as u64))
    } else if let Some(max_days) = args.cache_age {
        cache::CleanupMethod::MaxAge(Duration::from_secs((max_days as u64) * 24 * 60 * 60))
    } else {
        cache::CleanupMethod::None
    };

    // The number of allowed parallel podcast generations
    // WARNING: Currently more than 1 could cause issues where one podcast is generated multiple
    // times
    let podcast_generation_permit = Arc::new(Semaphore::new(1));

    // Create an OpenAPI service with the provided API and server URL.
    let api_service = OpenApiService::new(
        (feed::Router, content::Router, demo::Router),
        "feed2podcast",
        "0.1.0",
    )
    .description("Generate podcast feed from text based rss feeds using TTS\n\n[WebUI](/)")
    .server(args.url.clone())
    .url_prefix("/api");

    let webui_service = OpenApiService::new(webui::Router, "feed2podcast", "0.1.0");

    // Generate SwaggerUI documentation for the API.
    let docs = api_service.swagger_ui();

    let mut server = Route::new().nest("api", api_service);
    if !args.disable_docs {
        server = server.nest("docs", docs);
    }

    server = server.nest("/", webui_service);

    // Start the server with CORS middleware enabled.
    poem::Server::new(TcpListener::bind(format!("0.0.0.0:{}", args.port)))
        .run(
            server
                .with(Cors::new())
                .with(Tracing)
                .data(Feed2PodcastURLs {
                    base: args.url,
                    tts: args.tts_url,
                })
                .data(Feed2PodcastDirs {
                    cache: args.cache_dir,
                })
                .data(Feed2PodcastTTSConfig {
                    model: args.model,
                    voices: if args.voices.is_empty() {
                        None
                    } else {
                        Some(args.voices)
                    },
                })
                .data(podcast_generation_permit)
                .data(cache_cleanup_method),
        )
        .await
        .map_err(|e| eyre!(format!("Server failed with error: {e}")))
}
