use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../scripts/"]
pub struct Assets;
