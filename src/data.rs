
#[derive(Clone)]
pub struct Feed2PodcastURLs {
    pub base: String,
    pub tts: String,
}

#[derive(Clone)]
pub struct Feed2PodcastDirs {
    pub cache: String,
}

#[derive(Clone)]
pub struct Feed2PodcastTTSConfig {
    pub model: String,
    pub voices: Option<Vec<String>>,
}
