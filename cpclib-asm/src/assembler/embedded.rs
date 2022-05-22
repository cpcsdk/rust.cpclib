use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
#[prefix = "inner://"]
pub(crate) struct EmbeddedFiles;
