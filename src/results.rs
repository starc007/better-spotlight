use crate::apps::AppEntry;
use crate::files::FileEntry;

#[derive(Clone)]
pub enum SearchResult {
    Application(AppEntry),
    File(FileEntry),
}

impl SearchResult {
    pub fn name(&self) -> &str {
        match self {
            Self::Application(app) => &app.name,
            Self::File(file) => &file.name,
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::Application(app) => &app.path,
            Self::File(file) => &file.path,
        }
    }

    pub fn app(&self) -> Option<&AppEntry> {
        match self {
            Self::Application(app) => Some(app),
            Self::File(_) => None,
        }
    }

    pub fn subtitle(&self) -> Option<&str> {
        match self {
            Self::Application(_) => None,
            Self::File(file) => Some(&file.parent),
        }
    }
}
