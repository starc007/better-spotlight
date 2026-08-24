use gpui::Rgba;

pub const WINDOW_WIDTH: f32 = 720.0;
pub const WINDOW_HEIGHT: f32 = 460.0;
pub const MAX_VISIBLE_RESULTS: usize = 7;
pub const ICON_SIZE: f32 = 32.0;
pub const RESULT_ROW_HEIGHT: f32 = 44.0;

const fn hex_rgb(hex: u32) -> Rgba {
    let b = (hex & 0xff) as f32 / 255.0;
    let g = ((hex >> 8) & 0xff) as f32 / 255.0;
    let r = ((hex >> 16) & 0xff) as f32 / 255.0;
    Rgba { r, g, b, a: 1.0 }
}

impl Theme {
    pub const BG: Rgba = hex_rgb(0x17181c);
    pub const BORDER: Rgba = hex_rgb(0x2e3038);
    pub const INPUT_TEXT: Rgba = hex_rgb(0xf2f3f5);
    pub const PLACEHOLDER: Rgba = hex_rgb(0x6b6f7b);
    pub const RESULT_TEXT: Rgba = hex_rgb(0xc9ccd4);
    pub const RESULT_META: Rgba = hex_rgb(0x777b87);
    pub const SELECTED_BG: Rgba = hex_rgb(0x27408f);
    pub const SELECTED_TEXT: Rgba = hex_rgb(0xffffff);
    pub const CARET: Rgba = hex_rgb(0x8b8e97);
    pub const FOOTER_TEXT: Rgba = hex_rgb(0x565a66);
    pub const FALLBACK_TILES: [Rgba; 6] = [
        hex_rgb(0x3d5afe),
        hex_rgb(0x00897b),
        hex_rgb(0xd81b60),
        hex_rgb(0xfb8c00),
        hex_rgb(0x5e35b1),
        hex_rgb(0x546e7a),
    ];
}

pub struct Theme;

pub fn icon_fallback_color(name: &str) -> Rgba {
    let hash: u64 = name.bytes().map(|b| (b as u64).wrapping_mul(31)).sum();
    Theme::FALLBACK_TILES[(hash % Theme::FALLBACK_TILES.len() as u64) as usize]
}
