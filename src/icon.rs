use egui::IconData;

/// Generates a crisp 64x64 RGBA stock-chart icon for the window titlebar & taskbar.
pub fn build_window_icon() -> IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];

    let set_px = |buf: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8, a: u8| {
        if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
            return;
        }
        let idx = ((y as usize) * size + (x as usize)) * 4;
        let alpha = a as f32 / 255.0;
        let inv = 1.0 - alpha;
        buf[idx] = (r as f32 * alpha + buf[idx] as f32 * inv) as u8;
        buf[idx + 1] = (g as f32 * alpha + buf[idx + 1] as f32 * inv) as u8;
        buf[idx + 2] = (b as f32 * alpha + buf[idx + 2] as f32 * inv) as u8;
        buf[idx + 3] = buf[idx + 3].saturating_add(a);
    };

    // 1. Rounded dark slate background with subtle border
    let radius = 13.0f32;
    let max_c = (size - 1) as f32;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            let cx = fx.clamp(radius, max_c - radius);
            let cy = fy.clamp(radius, max_c - radius);
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            if dist <= radius {
                let t = fy / max_c;
                let bg_r = (16.0 + t * 8.0) as u8;
                let bg_g = (22.0 + t * 10.0) as u8;
                let bg_b = (34.0 + t * 14.0) as u8;
                let is_border = dist >= radius - 1.6;
                if is_border {
                    set_px(&mut rgba, x as i32, y as i32, 56, 78, 112, 255);
                } else {
                    set_px(&mut rgba, x as i32, y as i32, bg_r, bg_g, bg_b, 255);
                }
            }
        }
    }

    // 2. Subtle horizontal grid lines
    for &gy in &[18, 32, 46] {
        for x in 9..55 {
            set_px(&mut rgba, x, gy, 255, 255, 255, 20);
        }
    }

    // 3. Candlesticks (x_center, body_top, body_bottom, wick_top, wick_bottom, is_red)
    let candles = [
        (16, 36, 46, 32, 50, false),
        (27, 29, 41, 25, 45, true),
        (38, 25, 36, 20, 40, true),
        (49, 15, 28, 11, 33, true),
    ];
    for &(cx, bt, bb, wt, wb, is_up) in &candles {
        let (r, g, b) = if is_up {
            (52, 211, 130) // emerald green
        } else {
            (248, 92, 92) // coral red
        };
        for y in wt..=wb {
            set_px(&mut rgba, cx, y, r, g, b, 230);
            set_px(&mut rgba, cx - 1, y, r, g, b, 120);
        }
        for y in bt..=bb {
            for x in (cx - 3)..=(cx + 3) {
                set_px(&mut rgba, x, y, r, g, b, 255);
            }
        }
    }

    // 4. Rising trend line overlay
    let points = [(10.0f32, 47.0f32), (24.0, 39.0), (37.0, 30.0), (52.0, 14.0)];
    for w in points.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        let steps = 40;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let px = x0 + (x1 - x0) * t;
            let py = y0 + (y1 - y0) * t;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    set_px(
                        &mut rgba,
                        px.round() as i32 + dx,
                        py.round() as i32 + dy,
                        120,
                        215,
                        255,
                        220,
                    );
                }
            }
        }
    }

    IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
