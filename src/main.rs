use std::path::Path;

use poem::{
    EndpointExt, Route,
    listener::TcpListener,
    middleware::{Cors, Tracing},
};
use poem_openapi::OpenApiService;

use clap::Parser;
use eyre::{Result, eyre};

mod webui;

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
    let api_service = OpenApiService::new((), "feed2podcast", "0.1.0").server(args.url);

    // Generate SwaggerUI documentation for the API.
    let docs = api_service.swagger_ui();

    let mut server = Route::new().nest("/api", api_service);
    if !args.disable_docs {
        server = server.nest("docs", docs);
    }

    server = server.nest("/", webui::Router::get(static_files));

    // Start the server with CORS middleware enabled.
    poem::Server::new(TcpListener::bind(format!("0.0.0.0:{}", args.port)))
        .run(server.with(Cors::new()).with(Tracing))
        .await
        .map_err(|e| eyre!(format!("Server failed with error: {e}")))
}
