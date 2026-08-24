use std::path::PathBuf;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};

const DEFAULT_SHORTCUT: &str = "super+Space";
const SHORTCUT_ENV: &str = "BETTER_SPOTLIGHT_SHORTCUT";

pub struct ShortcutConfig {
    pub hotkey: HotKey,
    pub label: String,
    pub warning: Option<String>,
}

pub fn load_shortcut() -> ShortcutConfig {
    if let Ok(value) = std::env::var(SHORTCUT_ENV) {
        return parse_or_fallback(&value, SHORTCUT_ENV);
    }

    let Some(path) = config_path() else {
        return default_config(Some("Could not resolve the shortcut config path.".into()));
    };
    match std::fs::read_to_string(&path) {
        Ok(contents) => match shortcut_value(&contents) {
            Some(value) => parse_or_fallback(value, &path.to_string_lossy()),
            None => default_config(Some(format!(
                "No shortcut entry in {}. Using ⌘Space.",
                path.display()
            ))),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default_config(None),
        Err(error) => default_config(Some(format!(
            "Could not read {} ({error}). Using ⌘Space.",
            path.display()
        ))),
    }
}

pub fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Better Spotlight/config"))
}

fn shortcut_value(contents: &str) -> Option<&str> {
    contents.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        match line.split_once('=') {
            Some((key, value)) if key.trim() == "shortcut" => Some(value.trim()),
            None => Some(line),
            _ => None,
        }
    })
}

fn parse_or_fallback(value: &str, source: &str) -> ShortcutConfig {
    match parse_shortcut(value) {
        Ok(hotkey) => ShortcutConfig {
            label: shortcut_label(hotkey),
            hotkey,
            warning: None,
        },
        Err(error) => default_config(Some(format!(
            "Invalid shortcut in {source} ({error}). Using ⌘Space."
        ))),
    }
}

fn parse_shortcut(value: &str) -> Result<HotKey, String> {
    let hotkey = value.parse::<HotKey>().map_err(|error| error.to_string())?;
    if hotkey.mods.is_empty() {
        return Err("include at least one modifier".into());
    }
    Ok(hotkey)
}

fn default_config(warning: Option<String>) -> ShortcutConfig {
    let hotkey = DEFAULT_SHORTCUT.parse().expect("default shortcut is valid");
    ShortcutConfig {
        label: shortcut_label(hotkey),
        hotkey,
        warning,
    }
}

fn shortcut_label(hotkey: HotKey) -> String {
    let mut label = String::new();
    if hotkey.mods.contains(Modifiers::CONTROL) {
        label.push('⌃');
    }
    if hotkey.mods.contains(Modifiers::ALT) {
        label.push('⌥');
    }
    if hotkey.mods.contains(Modifiers::SHIFT) {
        label.push('⇧');
    }
    if hotkey.mods.contains(Modifiers::SUPER) {
        label.push('⌘');
    }
    let key = match hotkey.key {
        Code::Space => "Space".to_string(),
        _ => hotkey
            .key
            .to_string()
            .strip_prefix("Key")
            .unwrap_or("Key")
            .to_string(),
    };
    label.push_str(&key);
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_key_value_and_plain_shortcut_formats() {
        assert_eq!(
            shortcut_value("shortcut = super+shift+Space"),
            Some("super+shift+Space")
        );
        assert_eq!(shortcut_value("# comment\nalt+Space"), Some("alt+Space"));
    }

    #[test]
    fn requires_a_modifier_and_formats_mac_label() {
        assert!(parse_shortcut("Space").is_err());
        let hotkey = parse_shortcut("super+shift+KeyK").unwrap();
        assert_eq!(shortcut_label(hotkey), "⇧⌘K");
    }
}
