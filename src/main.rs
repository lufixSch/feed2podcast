use std::path::Path;

use poem::{
    EndpointExt, Route,
    listener::TcpListener,
    middleware::{Cors, Tracing},
};
use poem_openapi::OpenApiService;

use clap::Parser;
use eyre::{Result, eyre};

mod content;
mod feed;
mod webui;

mod data;
use data::Feed2PodcastURLs;

use crate::data::Feed2PodcastDirs;

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

    /// Shared files for WebUI.
    #[arg(
        short,
        long,
        help = "Location to the Shared files for the webui",
        env = "FEED2PODCAST_SHARED_DIR",
        default_value_t = String::from("./static")
    )]
    shared_dir: String,

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
        default_value_t = String::from("http://127.0.0.1:5000/v1")
    )]
    tts_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        unsafe {
            std::env::set_var("RUST_LOG", "poem=trace");
        }
    }
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let static_files = Path::new(&args.shared_dir);

    // Create an OpenAPI service with the provided API and server URL.
    let api_service = OpenApiService::new((feed::Router, content::Router), "feed2podcast", "0.1.0")
        .server(args.url.clone())
        .url_prefix("/api");

    // Generate SwaggerUI documentation for the API.
    let docs = api_service.swagger_ui();

    let mut server = Route::new().nest("api", api_service);
    if !args.disable_docs {
        server = server.nest("docs", docs);
    }

    server = server.nest("/", webui::Router::get(static_files));

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
                    shared: args.shared_dir,
                }),
        )
        .await
        .map_err(|e| eyre!(format!("Server failed with error: {e}")))
}
