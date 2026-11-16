//! The window's modules as the rest of the app calls them without the desktop: same names, an error.

/// `outside` without a desktop: nothing to open a link or a file with, nor to show it in.
pub mod outside {
    use std::path::Path;

    /// What is opened.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum What<'a> {
        /// A link (http, https, mailto…).
        Url(&'a str),
        /// A file in its program, or a folder in the file manager.
        Path(&'a Path),
        /// An item shown, selected, in its folder.
        Reveal(&'a Path),
    }

    pub fn open_url(url: &str) -> anyhow::Result<()> {
        open(What::Url(url))
    }

    pub fn open_allowed(allowed: &crate::opening::Allowed) -> anyhow::Result<()> {
        open(allowed.what())
    }

    fn open(_: What) -> anyhow::Result<()> {
        anyhow::bail!(tr!(
            "indisponible sans bureau",
            "not available without a desktop"
        ))
    }
}
