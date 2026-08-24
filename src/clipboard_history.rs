const MAX_ENTRIES: usize = 50;
const MAX_ENTRY_BYTES: usize = 100_000;

#[derive(Clone)]
pub struct ClipboardEntry {
    pub id: u64,
    pub text: String,
    preview: String,
    metadata: String,
}

impl ClipboardEntry {
    pub fn preview(&self) -> &str {
        &self.preview
    }

    pub fn metadata(&self) -> &str {
        &self.metadata
    }
}

#[derive(Default)]
pub struct ClipboardHistory {
    entries: Vec<ClipboardEntry>,
    next_id: u64,
    last_observed: Option<String>,
}

impl ClipboardHistory {
    pub fn capture(&mut self, text: String) -> bool {
        if self.last_observed.as_deref() == Some(text.as_str()) {
            return false;
        }
        self.last_observed = Some(text.clone());

        if text.trim().is_empty() || text.len() > MAX_ENTRY_BYTES {
            return false;
        }

        if self.entries.first().is_some_and(|entry| entry.text == text) {
            return false;
        }

        self.entries.retain(|entry| entry.text != text);
        self.next_id += 1;
        self.entries
            .insert(0, ClipboardEntry::new(self.next_id, text));
        self.entries.truncate(MAX_ENTRIES);
        true
    }

    pub fn search(&self, query: &str) -> Vec<ClipboardEntry> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return self.entries.clone();
        }

        self.entries
            .iter()
            .filter(|entry| entry.text.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    pub fn delete(&mut self, id: u64) -> bool {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != previous_len
    }

    pub fn clear(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        self.entries.clear();
        true
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl ClipboardEntry {
    fn new(id: u64, text: String) -> Self {
        let preview = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(160)
            .collect::<String>();
        let character_count = text.chars().count();
        let metadata = format!(
            "{character_count} {}",
            if character_count == 1 {
                "character"
            } else {
                "characters"
            }
        );
        Self {
            id,
            text,
            preview,
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_text_and_deduplicates_most_recent_first() {
        let mut history = ClipboardHistory::default();
        assert!(history.capture("first".to_string()));
        assert!(history.capture("second".to_string()));
        assert!(history.capture("first".to_string()));
        assert!(!history.capture("first".to_string()));

        let entries = history.search("");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "first");
        assert_eq!(entries[1].text, "second");
    }

    #[test]
    fn filters_case_insensitively_and_preserves_original_text() {
        let mut history = ClipboardHistory::default();
        history.capture("Launch Better Spotlight".to_string());
        history.capture("Unrelated note".to_string());

        let entries = history.search("SPOTLIGHT");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Launch Better Spotlight");
    }

    #[test]
    fn ignores_empty_and_oversized_entries_and_caps_history() {
        let mut history = ClipboardHistory::default();
        assert!(!history.capture("  \n".to_string()));
        assert!(!history.capture("x".repeat(MAX_ENTRY_BYTES + 1)));

        for index in 0..MAX_ENTRIES + 5 {
            history.capture(format!("entry {index}"));
        }
        let entries = history.search("");
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries[0].text, format!("entry {}", MAX_ENTRIES + 4));
        assert_eq!(entries.last().unwrap().text, "entry 5");
    }

    #[test]
    fn deletes_individual_entries_and_clears_history() {
        let mut history = ClipboardHistory::default();
        history.capture("one".to_string());
        history.capture("two".to_string());
        let id = history.search("")[0].id;

        assert!(history.delete(id));
        assert_eq!(history.search("").len(), 1);
        assert!(history.clear());
        assert!(history.is_empty());
        assert!(!history.clear());
    }

    #[test]
    fn deleted_current_clipboard_is_not_immediately_recaptured() {
        let mut history = ClipboardHistory::default();
        history.capture("current".to_string());
        let id = history.search("")[0].id;

        history.delete(id);
        assert!(!history.capture("current".to_string()));
        assert!(history.is_empty());
        assert!(history.capture("different".to_string()));
        assert!(history.capture("current".to_string()));
    }
}
