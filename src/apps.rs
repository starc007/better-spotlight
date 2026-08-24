use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use plist::Value;

#[derive(Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
    pub icon_path: Option<PathBuf>,
    pub icon: Option<Arc<RenderImage>>,
}

pub fn scan(dirs: &[String]) -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = dirs.iter().flat_map(|dir| scan_dir(dir)).collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps.dedup_by(|a, b| a.path == b.path);
    apps
}

pub fn launch(path: &str) -> std::io::Result<()> {
    let status = Command::new("open").arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "the macOS open command exited with {status}"
        )))
    }
}

fn scan_dir(dir: &str) -> Vec<AppEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let file_name = path.file_name()?.to_string_lossy().to_string();
            if !file_name.ends_with(".app") || !path.is_dir() {
                return None;
            }

            let bundle = bundle_metadata(&path);
            Some(AppEntry {
                name: bundle
                    .as_ref()
                    .and_then(|metadata| metadata.name.clone())
                    .unwrap_or_else(|| file_name.trim_end_matches(".app").to_string()),
                icon_path: bundle
                    .and_then(|metadata| metadata.icon_file)
                    .and_then(|icon| resolve_icon_path(&path, &icon))
                    .or_else(|| fallback_icon_path(&path)),
                icon: None,
                path: path.to_string_lossy().to_string(),
            })
        })
        .collect()
}

#[derive(Default)]
struct BundleMetadata {
    name: Option<String>,
    icon_file: Option<String>,
}

fn bundle_metadata(app_path: &Path) -> Option<BundleMetadata> {
    let plist = Value::from_file(app_path.join("Contents/Info.plist")).ok()?;
    let dictionary = plist.as_dictionary()?;
    let string = |key| {
        dictionary
            .get(key)
            .and_then(Value::as_string)
            .map(str::to_owned)
    };

    Some(BundleMetadata {
        name: string("CFBundleDisplayName").or_else(|| string("CFBundleName")),
        icon_file: string("CFBundleIconFile"),
    })
}

fn resolve_icon_path(app_path: &Path, icon_file: &str) -> Option<PathBuf> {
    let resources = app_path.join("Contents/Resources");
    let icon_path = resources.join(icon_file);
    if icon_path.is_file() {
        return Some(icon_path);
    }

    let with_extension = icon_path.with_extension("icns");
    with_extension.is_file().then_some(with_extension)
}

fn fallback_icon_path(app_path: &Path) -> Option<PathBuf> {
    std::fs::read_dir(app_path.join("Contents/Resources"))
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()?.to_str()? == "icns").then_some(path)
        })
        .max_by_key(|path| std::fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0))
}

/// Decodes an `.icns` file into a GPU-ready image. This is kept separate from
/// discovery so the UI can load only icons that are currently visible.
pub fn load_icon(icns_path: &Path) -> Option<Arc<RenderImage>> {
    let bytes = std::fs::read(icns_path).ok()?;
    let family = icns::IconFamily::read(&mut Cursor::new(bytes)).ok()?;

    let best = family
        .available_icons()
        .into_iter()
        .max_by_key(|icon_type| icon_type.pixel_width())?;
    let icon = family
        .get_icon_with_type(best)
        .ok()?
        .convert_to(icns::PixelFormat::RGBA);

    let rgba = RgbaImage::from_raw(icon.width(), icon.height(), icon.data().to_vec())?;
    Some(Arc::new(RenderImage::new(vec![Frame::new(rgba)])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_path_adds_icns_extension_when_bundle_omits_it() {
        let root = std::env::temp_dir().join(format!("better-spotlight-{}", std::process::id()));
        let resources = root.join("Contents/Resources");
        std::fs::create_dir_all(&resources).unwrap();
        let icon = resources.join("AppIcon.icns");
        std::fs::write(&icon, []).unwrap();

        assert_eq!(resolve_icon_path(&root, "AppIcon"), Some(icon));

        std::fs::remove_dir_all(root).unwrap();
    }
}
