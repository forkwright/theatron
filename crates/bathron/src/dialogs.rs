//! Native file-pick dialogs via [`rfd`].
//!
//! [`open_file`], [`open_files`], and [`save_file`] block the calling
//! thread until the user dismisses the dialog. They cannot be
//! unit-tested in CI; the pure-logic [`FileFilter`] is covered.
//!
//! [`rfd`]: https://docs.rs/rfd

/// File-type filter for the native dialog (e.g.
/// `FileFilter::new("Images", &["png", "jpg"])`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilter {
    /// Human-readable name shown in the dialog filter dropdown.
    pub name: String,
    /// File extensions, sans dot (e.g. `"png"`, not `".png"`).
    pub extensions: Vec<String>,
}

impl FileFilter {
    /// Construct a new filter.
    #[must_use]
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

#[cfg(not(test))]
fn build_dialog(filters: &[FileFilter]) -> rfd::FileDialog {
    let mut d = rfd::FileDialog::new();
    for f in filters {
        let exts: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
        d = d.add_filter(&f.name, &exts);
    }
    d
}

/// Open a native single-pick file dialog. Returns `None` if the user
/// cancels.
#[cfg(not(test))]
#[must_use]
pub fn open_file(filter: &[FileFilter]) -> Option<std::path::PathBuf> {
    build_dialog(filter).pick_file()
}

/// Open a native multi-pick file dialog. Returns an empty `Vec` if
/// the user cancels.
#[cfg(not(test))]
#[must_use]
pub fn open_files(filter: &[FileFilter]) -> Vec<std::path::PathBuf> {
    build_dialog(filter).pick_files().unwrap_or_default()
}

/// Open a native save-file dialog. Returns `None` if the user
/// cancels.
#[cfg(not(test))]
#[must_use]
pub fn save_file(default_name: &str, filter: &[FileFilter]) -> Option<std::path::PathBuf> {
    build_dialog(filter).set_file_name(default_name).save_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_constructor() {
        let f = FileFilter::new("Images", &["png", "jpg"]);
        assert_eq!(f.name, "Images");
        assert_eq!(f.extensions, vec!["png".to_string(), "jpg".to_string()]);
    }

    #[test]
    fn filter_empty_extensions() {
        let f = FileFilter::new("All", &[]);
        assert_eq!(f.name, "All");
        assert!(f.extensions.is_empty());
    }

    #[test]
    fn filter_equality() {
        let a = FileFilter::new("A", &["a"]);
        let b = FileFilter::new("A", &["a"]);
        let c = FileFilter::new("A", &["b"]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
