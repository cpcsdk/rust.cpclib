use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
#[prefix = "inner://"]
pub struct EmbeddedFiles;
