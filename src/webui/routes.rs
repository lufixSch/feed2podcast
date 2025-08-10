use std::path::Path;

use poem::{Route, endpoint::StaticFileEndpoint};

pub struct Router;

impl Router {
    pub fn get(static_files: &Path) -> Route {
        Route::new().at(
            "/",
            StaticFileEndpoint::new(static_files.join("index.html")),
        )
    }
}
