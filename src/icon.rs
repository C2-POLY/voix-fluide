//! The Voix Fluide logo, drawn in Rust (no SVG rasterizer needed).
//!
//! The geometry is the one of `assets/logo.svg` (viewBox 0 0 1024 1024): a damped
//! wave in one white stroke that calms into a straight line, then a blue caret,
//! on a dark rounded square. Every shape is a signed distance, so edges are
//! antialiased with a one pixel ramp.

/// Quadratic segments of the wave: (control, end), starting at `WAVE_START`.
const WAVE_START: (f32, f32) = (220.0, 512.0);
const WAVE_QUADS: [((f32, f32), (f32, f32)); 6] = [
    ((255.0, 152.0), (290.0, 512.0)),
    ((325.0, 812.0), (360.0, 512.0)),
    ((395.0, 292.0), (430.0, 512.0)),
    ((465.0, 652.0), (500.0, 512.0)),
    ((535.0, 442.0), (570.0, 512.0)),
    ((605.0, 536.0), (640.0, 512.0)),
];
/// The wave ends with a straight line to this point.
const WAVE_END: (f32, f32) = (690.0, 512.0);
const WAVE_HALF_WIDTH: f32 = 24.0;
const SAMPLES_PER_QUAD: usize = 32;

const BG_FILL: [f32; 3] = [0x0B as f32, 0x0F as f32, 0x1A as f32];
const BG_STROKE: [f32; 3] = [0x26 as f32, 0x2C as f32, 0x3C as f32];
const WAVE_COLOR: [f32; 3] = [0xF2 as f32, 0xF4 as f32, 0xF8 as f32];
const CARET_COLOR: [f32; 3] = [0x1A as f32, 0x75 as f32, 0xFF as f32];

type Pt = (f32, f32);

/// The wave as a polyline.
fn wave_polyline() -> Vec<Pt> {
    let mut pts = vec![WAVE_START];
    let mut p0 = WAVE_START;
    for &(c, p1) in &WAVE_QUADS {
        for i in 1..=SAMPLES_PER_QUAD {
            let t = i as f32 / SAMPLES_PER_QUAD as f32;
            let u = 1.0 - t;
            pts.push((
                u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0,
                u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1,
            ));
        }
        p0 = p1;
    }
    pts.push(WAVE_END);
    pts
}

fn dist_segment(p: Pt, a: Pt, b: Pt) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let (apx, apy) = (p.0 - a.0, p.1 - a.1);
    let len2 = abx * abx + aby * aby;
    let t = if len2 > 0.0 { ((apx * abx + apy * aby) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (dx, dy) = (apx - abx * t, apy - aby * t);
    (dx * dx + dy * dy).sqrt()
}

/// Signed distance to a rounded rectangle centered on (cx, cy) with half sizes (hx, hy).
fn sd_round_rect(p: Pt, c: Pt, h: Pt, r: f32) -> f32 {
    let qx = (p.0 - c.0).abs() - (h.0 - r);
    let qy = (p.1 - c.1).abs() - (h.1 - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// Draws `color` over the pixel with the given coverage (premultiplied accumulation).
fn over(px: &mut [f32; 4], color: [f32; 3], coverage: f32) {
    if coverage <= 0.0 {
        return;
    }
    let out_a = coverage + px[3] * (1.0 - coverage);
    for k in 0..3 {
        px[k] = (color[k] * coverage + px[k] * px[3] * (1.0 - coverage)) / out_a;
    }
    px[3] = out_a;
}

/// The logo as `size`×`size` RGBA pixels (not premultiplied). With the background it is
/// the whole 1024-unit icon; without, only the mark, cropped to 200..824 so it fills the image.
pub fn render(size: u32, with_background: bool) -> Vec<u8> {
    let (origin, span) = if with_background { (0.0, 1024.0) } else { (200.0, 624.0) };
    let scale = size as f32 / span; // pixels per design unit
    let wave = wave_polyline();
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &wave {
        x0 = x0.min(p.0);
        y0 = y0.min(p.1);
        x1 = x1.max(p.0);
        y1 = y1.max(p.1);
    }
    let margin = WAVE_HALF_WIDTH + 2.0;
    let (x0, y0, x1, y1) = (x0 - margin, y0 - margin, x1 + margin, y1 + margin);

    let mut out = vec![0u8; (size * size * 4) as usize];
    for py in 0..size {
        for px in 0..size {
            let p = (origin + (px as f32 + 0.5) / scale, origin + (py as f32 + 0.5) / scale);
            let mut acc = [0.0f32; 4];

            if with_background {
                let d = sd_round_rect(p, (512.0, 512.0), (412.0, 412.0), 185.0);
                over(&mut acc, BG_FILL, (0.5 - d * scale).clamp(0.0, 1.0));
                // 6 unit stroke centered on the edge.
                over(&mut acc, BG_STROKE, (0.5 - (d.abs() - 3.0) * scale).clamp(0.0, 1.0));
            }

            if p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1 {
                let d = wave.windows(2).map(|s| dist_segment(p, s[0], s[1])).fold(f32::MAX, f32::min) - WAVE_HALF_WIDTH;
                over(&mut acc, WAVE_COLOR, (0.5 - d * scale).clamp(0.0, 1.0));
            }

            let d = sd_round_rect(p, (762.0, 512.0), (20.0, 140.0), 20.0);
            over(&mut acc, CARET_COLOR, (0.5 - d * scale).clamp(0.0, 1.0));

            let i = ((py * size + px) * 4) as usize;
            for k in 0..3 {
                out[i + k] = acc[k].round().clamp(0.0, 255.0) as u8;
            }
            out[i + 3] = (acc[3] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(img: &[u8], size: u32, x: f32, y: f32) -> [u8; 4] {
        let i = ((y as u32 * size + x as u32) * 4) as usize;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn icon_with_background() {
        let img = render(64, true);
        assert_eq!(img.len(), 64 * 64 * 4);
        assert_eq!(pixel(&img, 64, 0.0, 0.0)[3], 0, "corner is transparent");
        // Background area left of the wave, away from the stroke.
        let bg = pixel(&img, 64, 10.0, 32.0);
        assert_eq!(bg[3], 255);
        assert!(bg[0] < 40 && bg[1] < 40 && bg[2] < 50, "dark background: {bg:?}");
        // Caret center.
        let c = pixel(&img, 64, 762.0 / 1024.0 * 64.0, 512.0 / 1024.0 * 64.0);
        assert!(c[2] > 200 && c[2] > c[0] + 100, "blue caret: {c:?}");
    }

    #[test]
    fn mark_has_no_background() {
        let img = render(64, false);
        // Top-left of the crop holds only background in the full icon.
        assert_eq!(pixel(&img, 64, 2.0, 2.0)[3], 0);
        // The straight tail of the wave is white and opaque.
        let tail = pixel(&img, 64, (665.0 - 200.0) / 624.0 * 64.0, (512.0 - 200.0) / 624.0 * 64.0);
        assert!(tail[3] == 255 && tail[0] > 200, "white tail: {tail:?}");
    }

    /// Writes the icon pictures to look at them: `cargo test write_icon_pngs -- --ignored`.
    #[test]
    #[ignore]
    fn write_icon_pngs() {
        for (size, bg, name) in [(512, true, "vf-icon-512"), (32, true, "vf-icon-32"), (112, false, "vf-mark-112")] {
            let img = image::RgbaImage::from_raw(size, size, render(size, bg)).unwrap();
            img.save(format!("/tmp/{name}.png")).unwrap();
        }
    }
}
