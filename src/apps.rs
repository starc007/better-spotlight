use std::io::Cursor;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use gpui::RenderImage;
use image::{Frame, RgbaImage};

#[derive(Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
    pub icon: Option<Arc<RenderImage>>,
}

pub fn scan(dirs: &[String]) -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = dirs.iter().flat_map(|dir| scan_dir(dir)).collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.path == b.path);
    apps
}

pub fn launch(path: &str) {
    let _ = Command::new("open").arg(path).spawn();
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

            Some(AppEntry {
                name: file_name.trim_end_matches(".app").to_string(),
                icon: load_icon(&path),
                path: path.to_string_lossy().to_string(),
            })
        })
        .collect()
}

/// Decodes the largest representation inside the app bundle's `.icns` file
/// into a GPU-ready image.
fn load_icon(app_path: &Path) -> Option<Arc<RenderImage>> {
    let resources = app_path.join("Contents/Resources");
    let icns_path = std::fs::read_dir(&resources)
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()?.to_str()? == "icns").then_some(path)
        })
        .max_by_key(|path| std::fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0))?;

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
