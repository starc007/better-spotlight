use crate::apps::AppEntry;
use crate::clipboard_history::ClipboardEntry;
use crate::files::FileEntry;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Calculator,
    Clipboard,
    Applications,
    Files,
}

impl ResultKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Calculator => "CALCULATOR",
            Self::Clipboard => "CLIPBOARD HISTORY",
            Self::Applications => "APPLICATIONS",
            Self::Files => "FILES & FOLDERS",
        }
    }
}

#[derive(Clone)]
pub enum SearchResult {
    Calculation { expression: String, result: String },
    Clipboard(ClipboardEntry),
    Application(AppEntry),
    File(FileEntry),
}

impl SearchResult {
    pub fn name(&self) -> &str {
        match self {
            Self::Calculation { result, .. } => result,
            Self::Clipboard(entry) => entry.preview(),
            Self::Application(app) => &app.name,
            Self::File(file) => &file.name,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Calculation { .. } | Self::Clipboard(_) => None,
            Self::Application(app) => Some(&app.path),
            Self::File(file) => Some(&file.path),
        }
    }

    pub fn app(&self) -> Option<&AppEntry> {
        match self {
            Self::Calculation { .. } | Self::Clipboard(_) => None,
            Self::Application(app) => Some(app),
            Self::File(_) => None,
        }
    }

    pub fn subtitle(&self) -> Option<&str> {
        match self {
            Self::Calculation { expression, .. } => Some(expression),
            Self::Clipboard(entry) => Some(entry.metadata()),
            Self::Application(_) => None,
            Self::File(file) => Some(&file.parent),
        }
    }

    pub fn calculation_result(&self) -> Option<&str> {
        match self {
            Self::Calculation { result, .. } => Some(result),
            _ => None,
        }
    }

    pub fn clipboard_entry(&self) -> Option<&ClipboardEntry> {
        match self {
            Self::Clipboard(entry) => Some(entry),
            _ => None,
        }
    }

    pub const fn kind(&self) -> ResultKind {
        match self {
            Self::Calculation { .. } => ResultKind::Calculator,
            Self::Clipboard(_) => ResultKind::Clipboard,
            Self::Application(_) => ResultKind::Applications,
            Self::File(_) => ResultKind::Files,
        }
    }
}
