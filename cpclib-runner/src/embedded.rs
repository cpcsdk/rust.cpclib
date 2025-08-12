use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "roms/"]
#[prefix = "roms://"]
pub struct EmbeddedRoms;
