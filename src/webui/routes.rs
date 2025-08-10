use std::path::Path;

use poem::{endpoint::{StaticFilesEndpoint}, Route};

pub struct Router;

impl Router {
    pub fn get(static_files: &Path) -> Route {
        Route::new().at(
            "/",
            StaticFilesEndpoint::new(static_files).index_file("index.html"),
        )
    }
}
