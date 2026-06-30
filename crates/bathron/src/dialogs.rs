//! Native file-pick + message dialogs via [`rfd`].
//!
//! Two surfaces:
//!
//! 1. **File dialogs.** [`open_file`], [`open_files`], and
//!    [`save_file`] for picking paths.
//! 2. **Message dialogs.** [`info`], [`warn`], [`error`] (one-button
//!    notification) and [`confirm`] (two-button yes/no decision).
//!
//! All functions block the calling thread until the user dismisses
//! the dialog. They cannot be unit-tested in CI; pure-logic types
//! ([`FileFilter`], [`MessageKind`]) are covered.
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
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn open_file(filter: &[FileFilter]) -> Option<std::path::PathBuf> {
    build_dialog(filter).pick_file()
}

/// Open a native multi-pick file dialog. Returns an empty `Vec` if
/// the user cancels.
#[cfg(not(test))]
#[must_use]
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn open_files(filter: &[FileFilter]) -> Vec<std::path::PathBuf> {
    // kanon:ignore RUST/no-result-unwrap-or-default -- `pick_files` returns Option<Vec<PathBuf>> (None == user canceled); collapse to empty Vec is the documented behavior.
    build_dialog(filter).pick_files().unwrap_or_default()
}

/// Open a native save-file dialog. Returns `None` if the user
/// cancels.
#[cfg(not(test))]
#[must_use]
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn save_file(default_name: &str, filter: &[FileFilter]) -> Option<std::path::PathBuf> {
    build_dialog(filter).set_file_name(default_name).save_file()
}

/// Severity for a message dialog.
///
/// Maps directly onto [`rfd::MessageLevel`]. Drives the icon and
/// stylistic register the OS uses to render the dialog (info icon,
/// warning triangle, error cross).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MessageKind {
    /// Informational. Default.
    #[default]
    Info,
    /// Warning — non-fatal but operator should attend.
    Warning,
    /// Error — operation failed.
    Error,
}

#[cfg(not(test))]
impl MessageKind {
    fn to_rfd(self) -> rfd::MessageLevel {
        match self {
            Self::Info => rfd::MessageLevel::Info,
            Self::Warning => rfd::MessageLevel::Warning,
            Self::Error => rfd::MessageLevel::Error,
        }
    }
}

/// Show an informational message dialog with an OK button. Blocks
/// until dismissed.
#[cfg(not(test))]
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn info(title: &str, message: &str) {
    show_message(MessageKind::Info, title, message);
}

/// Show a warning message dialog with an OK button. Blocks until
/// dismissed.
#[cfg(not(test))]
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn warn(title: &str, message: &str) {
    show_message(MessageKind::Warning, title, message);
}

/// Show an error message dialog with an OK button. Blocks until
/// dismissed.
#[cfg(not(test))]
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn error(title: &str, message: &str) {
    show_message(MessageKind::Error, title, message);
}

/// Show a yes/no confirmation dialog. Returns `true` if the user
/// clicked Yes, `false` for No or any other dismissal (Esc, window
/// close button). Blocks until dismissed.
#[cfg(not(test))]
#[must_use]
// kanon:ignore RUST/pub-visibility -- bathron dialog API is consumed by external desktop crates
pub fn confirm(title: &str, message: &str) -> bool {
    let result = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title(title)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    matches!(result, rfd::MessageDialogResult::Yes)
}

#[cfg(not(test))]
fn show_message(kind: MessageKind, title: &str, message: &str) {
    rfd::MessageDialog::new()
        .set_level(kind.to_rfd())
        .set_title(title)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
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

    #[test]
    fn message_kind_default_is_info() {
        assert_eq!(MessageKind::default(), MessageKind::Info);
    }

    #[test]
    fn message_kind_variants_are_distinct() {
        assert_ne!(MessageKind::Info, MessageKind::Warning);
        assert_ne!(MessageKind::Warning, MessageKind::Error);
        assert_ne!(MessageKind::Info, MessageKind::Error);
    }

    #[test]
    fn message_kind_is_copy() {
        let kind = MessageKind::Warning;
        let copied = kind;

        assert_eq!(kind, copied);
    }
}
