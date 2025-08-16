use std::path::Path;

use poem::{Route, endpoint::StaticFilesEndpoint};

pub struct Router;

impl Router {
    pub fn get(static_files: &Path) -> Route {
        Route::new()
            .at(
                "/",
                StaticFilesEndpoint::new(static_files).index_file("index.html"),
            )
            .nest(
                "/demo",
                StaticFilesEndpoint::new(static_files).index_file("demo.html"),
            )
    }
}
