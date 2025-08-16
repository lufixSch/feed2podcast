use poem_openapi::{ApiResponse, Tags, payload::Binary};

/// OpenAPI Category Tags for API endpoints
#[derive(Tags)]
pub enum CategoryTags {
    /// Podcast Feeds
    Feed,

    /// Voice demos and similar
    Demo,
}

#[derive(Debug, ApiResponse)]
pub enum DownloadFileResponse {
    #[oai(status = 200)]
    Audio(Binary<Vec<u8>>, #[oai(header = "content-type")] String),
}
