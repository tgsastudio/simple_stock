use egui::IconData;

fn dist_to_segment(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq > 0.0 {
        (((px - x0) * dx + (py - y0) * dy) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let proj_x = x0 + t * dx;
    let proj_y = y0 + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}

/// Generates a crisp 64x64 auspicious crimson-red icon with white rising trend arrow.
pub fn build_window_icon() -> IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];

    let segments = [
        (13.0f32, 44.0f32, 25.0f32, 31.0f32),
        (25.0, 31.0, 34.0, 38.0),
        (34.0, 38.0, 49.0, 20.0),
        (39.0, 20.0, 49.0, 20.0),
        (49.0, 20.0, 49.0, 30.0),
    ];

    let pad = 2.0f32;
    let radius = 15.0f32;
    let min_c = pad + radius;
    let max_c = (size as f32) - pad - radius;
    let half_stroke = 2.9f32;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let cx = fx.clamp(min_c, max_c);
            let cy = fy.clamp(min_c, max_c);
            let bg_dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() - radius;
            let bg_alpha = (0.5 - bg_dist).clamp(0.0, 1.0);
            if bg_alpha <= 0.0 {
                continue;
            }

            let t = ((fx + fy) / (2.0 * size as f32)).clamp(0.0, 1.0);
            let mut r = 225.0 - t * 57.0;
            let mut g = 45.0 - t * 23.0;
            let mut b = 57.0 - t * 21.0;

            let mut min_d = f32::MAX;
            for &(x0, y0, x1, y1) in &segments {
                let d = dist_to_segment(fx, fy, x0, y0, x1, y1);
                if d < min_d {
                    min_d = d;
                }
            }
            let fg_alpha = (half_stroke + 0.6 - min_d).clamp(0.0, 1.0);
            if fg_alpha > 0.0 {
                r = 255.0 * fg_alpha + r * (1.0 - fg_alpha);
                g = 252.0 * fg_alpha + g * (1.0 - fg_alpha);
                b = 245.0 * fg_alpha + b * (1.0 - fg_alpha);
            }

            let idx = (y * size + x) * 4;
            rgba[idx] = r.round() as u8;
            rgba[idx + 1] = g.round() as u8;
            rgba[idx + 2] = b.round() as u8;
            rgba[idx + 3] = (bg_alpha * 255.0).round() as u8;
        }
    }

    IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
