use egui::{FontData, FontDefinitions, FontFamily};
use std::fs;
use std::sync::Arc;

/// Dynamically loads a Windows system Traditional Chinese font so we don't bundle a huge font file.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let candidates = [
        r"C:\Windows\Fonts\msjh.ttc",     // Microsoft JhengHei (微軟正黑體)
        r"C:\Windows\Fonts\msjh.ttf",
        r"C:\Windows\Fonts\msyh.ttc",     // Microsoft YaHei fallback
        r"C:\Windows\Fonts\mingliu.ttc",  // MingLiU fallback
    ];

    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            fonts.font_data.insert(
                "win_cjk".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "win_cjk".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .push("win_cjk".to_owned());
            break;
        }
    }

    ctx.set_fonts(fonts);
}
