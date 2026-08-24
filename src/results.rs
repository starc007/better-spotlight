use crate::apps::AppEntry;
use crate::files::FileEntry;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Calculator,
    Applications,
    Files,
}

impl ResultKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Calculator => "CALCULATOR",
            Self::Applications => "APPLICATIONS",
            Self::Files => "FILES & FOLDERS",
        }
    }
}

#[derive(Clone)]
pub enum SearchResult {
    Calculation { expression: String, result: String },
    Application(AppEntry),
    File(FileEntry),
}

impl SearchResult {
    pub fn name(&self) -> &str {
        match self {
            Self::Calculation { result, .. } => result,
            Self::Application(app) => &app.name,
            Self::File(file) => &file.name,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Calculation { .. } => None,
            Self::Application(app) => Some(&app.path),
            Self::File(file) => Some(&file.path),
        }
    }

    pub fn app(&self) -> Option<&AppEntry> {
        match self {
            Self::Calculation { .. } => None,
            Self::Application(app) => Some(app),
            Self::File(_) => None,
        }
    }

    pub fn subtitle(&self) -> Option<&str> {
        match self {
            Self::Calculation { expression, .. } => Some(expression),
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

    pub const fn kind(&self) -> ResultKind {
        match self {
            Self::Calculation { .. } => ResultKind::Calculator,
            Self::Application(_) => ResultKind::Applications,
            Self::File(_) => ResultKind::Files,
        }
    }
}
