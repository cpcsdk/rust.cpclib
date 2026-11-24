//! The files of `assets/`, which a source includes as `inner://NAME`.
//!
//! With the `embedded-files` feature they are in the library; without it there are none (several
//! are third-party routines whose licences are not all stated), and an `inner://` include fails.

#[cfg(feature = "embedded-files")]
mod files {
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "assets/"]
    #[prefix = "inner://"]
    pub struct EmbeddedFiles;
}

#[cfg(not(feature = "embedded-files"))]
mod files {
    use std::borrow::Cow;

    /// No file embedded (the `embedded-files` feature is off): the same functions as with it.
    pub struct EmbeddedFiles;

    impl EmbeddedFiles {
        pub fn get(_file_path: &str) -> Option<rust_embed::EmbeddedFile> {
            None
        }

        pub fn iter() -> impl Iterator<Item = Cow<'static, str>> {
            std::iter::empty()
        }
    }
}

pub use files::EmbeddedFiles;

#[cfg(test)]
mod tests {
    use super::EmbeddedFiles;

    #[test]
    fn the_files_are_embedded_with_the_feature_only() {
        let found = EmbeddedFiles::get("inner://deexo.asm").is_some();
        assert_eq!(found, cfg!(feature = "embedded-files"));
        assert_eq!(EmbeddedFiles::iter().next().is_some(), cfg!(feature = "embedded-files"));
    }
}
