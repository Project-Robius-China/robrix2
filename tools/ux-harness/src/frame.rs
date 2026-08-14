//! Pixel-level analysis of captured frames.
//!
//! Everything here works on a decoded RGBA8 buffer. The point is to turn a
//! screenshot into claims a reviewer would otherwise have to make by eye:
//! "this screen is blank", "nothing changed when I clicked", "this label does
//! not have enough contrast against what is behind it".

use std::path::Path;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// RGBA8, row-major.
    pub data: Vec<u8>,
}

impl Frame {
    pub fn load(path: &Path) -> Result<Frame, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("open {}: {e}", path.display()))?;
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("png header {}: {e}", path.display()))?;
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| format!("png {} has no decodable size", path.display()))?;
        let mut buf = vec![0u8; size];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|e| format!("png decode {}: {e}", path.display()))?;

        let channels = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Indexed => {
                return Err(format!("png {} is indexed-color", path.display()))
            }
        };
        if info.bit_depth != png::BitDepth::Eight {
            return Err(format!("png {} is not 8-bit", path.display()));
        }

        let px = (info.width as usize) * (info.height as usize);
        let mut data = vec![0u8; px * 4];
        for i in 0..px {
            let s = i * channels;
            let d = i * 4;
            match channels {
                4 => data[d..d + 4].copy_from_slice(&buf[s..s + 4]),
                3 => {
                    data[d] = buf[s];
                    data[d + 1] = buf[s + 1];
                    data[d + 2] = buf[s + 2];
                    data[d + 3] = 255;
                }
                2 => {
                    data[d] = buf[s];
                    data[d + 1] = buf[s];
                    data[d + 2] = buf[s];
                    data[d + 3] = buf[s + 1];
                }
                _ => {
                    data[d] = buf[s];
                    data[d + 1] = buf[s];
                    data[d + 2] = buf[s];
                    data[d + 3] = 255;
                }
            }
        }
        Ok(Frame { width: info.width, height: info.height, data })
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let i = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }

    /// Fraction of the frame occupied by its single most common color.
    /// A login screen sits around 0.6–0.9; a blank/white screen is ~1.0.
    pub fn dominant_ratio(&self) -> f64 {
        let mut counts = std::collections::HashMap::new();
        let total = (self.width as usize) * (self.height as usize);
        if total == 0 {
            return 1.0;
        }
        // Sample rather than count every pixel — a 2560x1600 frame is 4M pixels
        // and the ratio is stable well before that.
        let step = ((total / 200_000).max(1)) as usize;
        let mut sampled = 0usize;
        for i in (0..total).step_by(step) {
            let p = &self.data[i * 4..i * 4 + 4];
            let key = (p[0] / 8, p[1] / 8, p[2] / 8);
            *counts.entry(key).or_insert(0usize) += 1;
            sampled += 1;
        }
        let max = counts.values().copied().max().unwrap_or(0);
        if sampled == 0 {
            1.0
        } else {
            max as f64 / sampled as f64
        }
    }

    /// Number of distinct quantised colors — a proxy for "is there content".
    pub fn distinct_colors(&self) -> usize {
        let mut set = std::collections::HashSet::new();
        let total = (self.width as usize) * (self.height as usize);
        let step = ((total / 200_000).max(1)) as usize;
        for i in (0..total).step_by(step) {
            let p = &self.data[i * 4..i * 4 + 4];
            set.insert((p[0] / 8, p[1] / 8, p[2] / 8));
        }
        set.len()
    }

    /// Fraction of sampled pixels that differ between two frames.
    pub fn diff_ratio(&self, other: &Frame) -> f64 {
        if self.width != other.width || self.height != other.height {
            return 1.0;
        }
        let total = (self.width as usize) * (self.height as usize);
        if total == 0 {
            return 0.0;
        }
        let step = ((total / 200_000).max(1)) as usize;
        let mut changed = 0usize;
        let mut sampled = 0usize;
        for i in (0..total).step_by(step) {
            let a = &self.data[i * 4..i * 4 + 3];
            let b = &other.data[i * 4..i * 4 + 3];
            let d = (a[0] as i32 - b[0] as i32).abs()
                + (a[1] as i32 - b[1] as i32).abs()
                + (a[2] as i32 - b[2] as i32).abs();
            if d > 12 {
                changed += 1;
            }
            sampled += 1;
        }
        if sampled == 0 {
            0.0
        } else {
            changed as f64 / sampled as f64
        }
    }

    /// Contrast of a text region, estimated the way a reader experiences it:
    /// the most common color in the rect is the background, and the pixel
    /// furthest from it in luminance is the ink.
    ///
    /// Returns `None` when the rect holds no discernible ink (empty label,
    /// icon-only, or entirely off-frame).
    pub fn region_contrast(&self, x: i64, y: i64, w: i64, h: i64) -> Option<ContrastSample> {
        if w <= 0 || h <= 0 {
            return None;
        }
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x + w).min(self.width as i64)).max(0) as u32;
        let y1 = ((y + h).min(self.height as i64)).max(0) as u32;
        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        let mut counts: std::collections::HashMap<(u8, u8, u8), usize> = std::collections::HashMap::new();
        let mut pixels: Vec<((u8, u8, u8), f64)> = Vec::new();
        for yy in y0..y1 {
            for xx in x0..x1 {
                let p = self.pixel(xx, yy);
                let rgb = (p[0], p[1], p[2]);
                *counts.entry(rgb).or_insert(0) += 1;
                pixels.push((rgb, relative_luminance(p[0], p[1], p[2])));
            }
        }
        let total = pixels.len();
        if total == 0 {
            return None;
        }
        // Background = the most common color. On any real control the surface
        // outweighs the glyphs.
        let (bg, bg_count) = counts
            .iter()
            .max_by_key(|(_, c)| **c)
            .map(|(k, c)| (*k, *c))?;
        if bg_count == total {
            return None; // one flat color: no text here
        }
        let bg_lum = relative_luminance(bg.0, bg.1, bg.2);
        let bg_is_light = bg_lum > 0.5;

        // Ink is estimated by percentile rather than by picking a discrete
        // color. At this type scale (9–17px) most glyph pixels are partially
        // covered, so the *most common* non-background color is usually an
        // antialiasing blend, which reads as better contrast than the reader
        // actually gets. The 2nd percentile in the direction text must go is
        // both robust to that and stable across renderers.
        let mut lums: Vec<f64> = pixels.iter().map(|(_, l)| *l).collect();
        lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((total as f64) * 0.02) as usize;
        let ink_lum = if bg_is_light {
            lums[idx.min(total - 1)]
        } else {
            lums[(total - 1 - idx.min(total - 1)).min(total - 1)]
        };

        // Text has to be darker than a light surface (or lighter than a dark
        // one). If it is not, the rect holds no glyphs — which is what a
        // stale rect from a screen that is not presented looks like.
        if (bg_is_light && ink_lum >= bg_lum) || (!bg_is_light && ink_lum <= bg_lum) {
            return None;
        }

        // Coverage = pixels that are more ink than background. Below ~1% there
        // is nothing to read, and a near-uniform rect would otherwise report a
        // dramatic-looking ratio built out of a handful of stray pixels.
        let midpoint = (ink_lum + bg_lum) / 2.0;
        let covered = pixels
            .iter()
            .filter(|(_, l)| if bg_is_light { *l < midpoint } else { *l > midpoint })
            .count();
        let ink_coverage = covered as f64 / total as f64;
        if ink_coverage < 0.01 {
            return None;
        }

        // Name the pixel whose luminance is closest to the one the ratio was
        // computed from, so the evidence line and the number agree.
        let ink = pixels
            .iter()
            .min_by(|(_, a), (_, b)| {
                (a - ink_lum)
                    .abs()
                    .partial_cmp(&(b - ink_lum).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(c, _)| *c)
            .unwrap_or(bg);

        Some(ContrastSample {
            ratio: contrast_ratio(ink_lum, bg_lum),
            ink,
            background: bg,
            ink_coverage,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ContrastSample {
    pub ratio: f64,
    pub ink: (u8, u8, u8),
    pub background: (u8, u8, u8),
    pub ink_coverage: f64,
}

pub fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    fn channel(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

pub fn contrast_ratio(l1: f64, l2: f64) -> f64 {
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

/// WCAG contrast between two hex colors, e.g. `contrast_hex("#16233B", "#FFFFFF")`.
pub fn contrast_hex(fg: &str, bg: &str) -> Option<f64> {
    let f = parse_hex(fg)?;
    let b = parse_hex(bg)?;
    Some(contrast_ratio(
        relative_luminance(f.0, f.1, f.2),
        relative_luminance(b.0, b.1, b.2),
    ))
}

pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim().trim_start_matches("#x").trim_start_matches('#');
    if s.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}
