//! Finding color values in text (`#rgb` hex, `rgb()`, `hsl()`) and writing
//! them back in the same format after the color picker changes them.

use std::ops::Range;

/// Past this size, files get no swatches, so every keystroke stays quick.
pub const MAX_SCAN_BYTES: usize = 2 * 1024 * 1024;
/// The most colors one document reports.
const MAX_COLORS: usize = 10_000;
/// A function color longer than this isn't a color (`rgb(` followed by junk).
const MAX_FUNCTION_LEN: usize = 64;

/// Red, green, blue, and alpha, each 0 to 1.
pub type Rgba = [f32; 4];

#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    /// `digits` is 3, 4, 6, or 8.
    Hex { digits: u8, upper: bool },
    Function {
        /// The name as written, like `rgba` or `HSL`.
        name: String,
        hsl: bool,
        /// `rgb(1, 2, 3)` rather than `rgb(1 2 3)`.
        commas: bool,
        /// The value was written with an alpha part.
        alpha: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorValue {
    /// Byte range in the whole document.
    pub range: Range<usize>,
    pub rgba: Rgba,
    pub format: Format,
}

/// One line's colors, with the line's byte range (newline excluded).
#[derive(Debug, Clone, PartialEq)]
pub struct ColorLine {
    pub start: usize,
    pub end: usize,
    pub colors: Vec<ColorValue>,
}

/// Every line that has a color value in it, in order.
pub fn scan(text: &str) -> Vec<ColorLine> {
    if text.len() > MAX_SCAN_BYTES {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut total = 0;
    let mut start = 0;
    for line in text.split('\n') {
        let colors = find_in_line(line, start);
        if !colors.is_empty() {
            total += colors.len();
            lines.push(ColorLine {
                start,
                end: start + line.len(),
                colors,
            });
            if total >= MAX_COLORS {
                break;
            }
        }
        start += line.len() + 1;
    }
    lines
}

/// The color whose range contains `offset` (its edges included).
pub fn at_offset(lines: &[ColorLine], offset: usize) -> Option<&ColorValue> {
    let line = lines.iter().find(|line| (line.start..=line.end).contains(&offset))?;
    line.colors
        .iter()
        .find(|color| (color.range.start..=color.range.end).contains(&offset))
}

fn find_in_line(line: &str, base: usize) -> Vec<ColorValue> {
    let bytes = line.as_bytes();
    let mut colors = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let before = i.checked_sub(1).map(|j| bytes[j]);
        let found = match bytes[i] {
            b'#' => parse_hex(line, i, before),
            b'r' | b'R' | b'h' | b'H' => parse_function(line, i, before),
            _ => None,
        };
        match found {
            Some((len, rgba, format)) => {
                colors.push(ColorValue {
                    range: base + i..base + i + len,
                    rgba,
                    format,
                });
                i += len;
            }
            None => i += 1,
        }
    }
    colors
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `#` plus 3, 4, 6, or 8 hex digits, standing alone (not `&#123;` or `a#fff`).
fn parse_hex(line: &str, at: usize, before: Option<u8>) -> Option<(usize, Rgba, Format)> {
    if before.is_some_and(|b| is_word(b) || b == b'&' || b == b'#') {
        return None;
    }
    let digits = line[at + 1..].bytes().take_while(u8::is_ascii_hexdigit).count();
    if !matches!(digits, 3 | 4 | 6 | 8) {
        return None;
    }
    let end = at + 1 + digits;
    if line.as_bytes().get(end).is_some_and(|&b| is_word(b)) {
        return None;
    }
    let hex = &line[at + 1..end];
    let channel = |i: usize, width: usize| {
        let value = u8::from_str_radix(&hex[i * width..(i + 1) * width], 16).ok()?;
        Some(if width == 1 { value * 17 } else { value } as f32 / 255.)
    };
    let width = if digits <= 4 { 1 } else { 2 };
    let alpha = if digits == 4 || digits == 8 { channel(3, width)? } else { 1. };
    let rgba = [channel(0, width)?, channel(1, width)?, channel(2, width)?, alpha];
    let upper = hex.bytes().any(|b| b.is_ascii_uppercase());
    Some((1 + digits, rgba, Format::Hex { digits: digits as u8, upper }))
}

/// `rgb(...)`, `rgba(...)`, `hsl(...)`, or `hsla(...)` with plain numbers.
fn parse_function(line: &str, at: usize, before: Option<u8>) -> Option<(usize, Rgba, Format)> {
    if before.is_some_and(|b| is_word(b) || b == b'-') {
        return None;
    }
    let rest = &line[at..];
    let open = rest.bytes().take(5).position(|b| b == b'(')?;
    let name = &rest[..open];
    let lower = name.to_ascii_lowercase();
    let hsl = match lower.as_str() {
        "rgb" | "rgba" => false,
        "hsl" | "hsla" => true,
        _ => return None,
    };
    let close = rest[open..].bytes().take(MAX_FUNCTION_LEN).position(|b| b == b')')? + open;
    let inner = &rest[open + 1..close];
    let commas = inner.contains(',');
    let parts: Vec<&str> = inner
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }

    let alpha = match parts.get(3) {
        Some(part) => fraction(part, 1.)?,
        None => 1.,
    };
    let rgba = if hsl {
        let hue = number(parts[0].trim_end_matches("deg"))?.rem_euclid(360.) / 360.;
        let [r, g, b] = hsl_to_rgb(hue, fraction(parts[1], 100.)?, fraction(parts[2], 100.)?);
        [r, g, b, alpha]
    } else {
        [fraction(parts[0], 255.)?, fraction(parts[1], 255.)?, fraction(parts[2], 255.)?, alpha]
    };
    let format = Format::Function {
        name: name.to_string(),
        hsl,
        commas,
        alpha: parts.len() == 4 || lower.ends_with('a'),
    };
    Some((close + 1, rgba, format))
}

fn number(text: &str) -> Option<f32> {
    let value: f32 = text.parse().ok()?;
    value.is_finite().then_some(value)
}

/// `50%` is half; a plain number is divided by `scale`. Clamped to 0..=1.
fn fraction(text: &str, scale: f32) -> Option<f32> {
    let value = match text.strip_suffix('%') {
        Some(percent) => number(percent)? / 100.,
        None => number(text)? / scale,
    };
    Some(value.clamp(0., 1.))
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let q = if l < 0.5 { l * (1. + s) } else { l + s - l * s };
    let p = 2. * l - q;
    let channel = |t: f32| {
        let t = t.rem_euclid(1.);
        if t < 1. / 6. {
            p + (q - p) * 6. * t
        } else if t < 0.5 {
            q
        } else if t < 2. / 3. {
            p + (q - p) * (2. / 3. - t) * 6.
        } else {
            p
        }
    };
    [channel(h + 1. / 3.), channel(h), channel(h - 1. / 3.)]
}

fn rgb_to_hsl([r, g, b]: [f32; 3]) -> [f32; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.;
    let d = max - min;
    if d == 0. {
        return [0., 0., l];
    }
    let s = if l > 0.5 { d / (2. - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6. } else { 0. }
    } else if max == g {
        (b - r) / d + 2.
    } else {
        (r - g) / d + 4.
    };
    [h / 6., s, l]
}

/// Writes `rgba` the way `format` was written. Alpha is added when the new
/// color needs it, and short hex stays short when the color still fits.
pub fn format(rgba: Rgba, format: &Format) -> String {
    let byte = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    let opaque = byte(rgba[3]) == 255;
    match format {
        Format::Hex { digits, upper } => {
            let bytes = rgba.map(byte);
            let short_fits = bytes.iter().all(|b| b % 17 == 0);
            let with_alpha = !opaque || matches!(digits, 4 | 8);
            let count = if with_alpha { 4 } else { 3 };
            let hex: String = if short_fits && matches!(digits, 3 | 4) {
                bytes[..count].iter().map(|b| format!("{:x}", b / 17)).collect()
            } else {
                bytes[..count].iter().map(|b| format!("{b:02x}")).collect()
            };
            let hex = if *upper { hex.to_ascii_uppercase() } else { hex };
            format!("#{hex}")
        }
        Format::Function {
            name,
            hsl,
            commas,
            alpha,
        } => {
            let channels = if *hsl {
                let [h, s, l] = rgb_to_hsl([rgba[0], rgba[1], rgba[2]]);
                [
                    format!("{}", (h * 360.).round() as u32 % 360),
                    format!("{}%", (s * 100.).round()),
                    format!("{}%", (l * 100.).round()),
                ]
            } else {
                [rgba[0], rgba[1], rgba[2]].map(|value| byte(value).to_string())
            };
            let mut name = name.clone();
            let with_alpha = *alpha || !opaque;
            // `rgb(1, 2, 3, 0.5)` isn't valid in older CSS, so commas get `rgba`.
            if with_alpha && *commas && !name.to_ascii_lowercase().ends_with('a') {
                let a = if name.bytes().all(|b| b.is_ascii_uppercase()) { "A" } else { "a" };
                name.push_str(a);
            }
            let alpha_text = format!("{}", (rgba[3].clamp(0., 1.) * 100.).round() / 100.);
            match (*commas, with_alpha) {
                (true, true) => format!("{name}({}, {alpha_text})", channels.join(", ")),
                (true, false) => format!("{name}({})", channels.join(", ")),
                (false, true) => format!("{name}({} / {alpha_text})", channels.join(" ")),
                (false, false) => format!("{name}({})", channels.join(" ")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<&str> {
        scan(text)
            .iter()
            .flat_map(|line| line.colors.iter().map(|color| &text[color.range.clone()]))
            .collect()
    }

    fn only(text: &str) -> ColorValue {
        let lines = scan(text);
        assert_eq!(lines.len(), 1, "{text:?}");
        assert_eq!(lines[0].colors.len(), 1, "{text:?}");
        lines[0].colors[0].clone()
    }

    fn close(a: Rgba, b: Rgba) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01)
    }

    #[test]
    fn finds_every_hex_length() {
        let text = "a: #f00; b: #f008; c: #ff0000; d: #ff000080;";
        assert_eq!(found(text), ["#f00", "#f008", "#ff0000", "#ff000080"]);
        assert!(close(only("#f008").rgba, [1., 0., 0., 0.533]));
        assert!(close(only("#42a4cf").rgba, [0.259, 0.643, 0.812, 1.]));
    }

    #[test]
    fn skips_hex_that_isnt_a_color() {
        assert!(found("#12345 #1234567 #ggg").is_empty(), "wrong lengths and non-hex");
        assert!(found("&#123; a#fff #fffz ##fff").is_empty(), "entities and glued words");
        assert!(found("#[derive(Debug)]").is_empty());
    }

    #[test]
    fn finds_rgb_and_hsl_functions() {
        let text = "rgb(255, 0, 0) rgba(0,0,0,.5) rgb(0 128 255 / 50%) hsl(120deg 100% 50%) HSLA(0, 0%, 100%, 1)";
        assert_eq!(
            found(text),
            ["rgb(255, 0, 0)", "rgba(0,0,0,.5)", "rgb(0 128 255 / 50%)", "hsl(120deg 100% 50%)", "HSLA(0, 0%, 100%, 1)"]
        );
        assert!(close(only("hsl(120, 100%, 50%)").rgba, [0., 1., 0., 1.]));
        assert!(close(only("rgb(0 128 255 / 50%)").rgba, [0., 0.502, 1., 0.5]));
    }

    #[test]
    fn skips_functions_that_arent_colors() {
        assert!(found("rgb(var(--x)) rgb(1, 2) myrgb(1,2,3) --rgb(1,2,3) rgb(1, 2, 3").is_empty());
    }

    #[test]
    fn ranges_are_document_offsets_with_line_bounds() {
        let lines = scan("one\n  color: #abc;\nnone\nx #000 #fff\n");
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].start, lines[0].end), (4, 18));
        assert_eq!(lines[0].colors[0].range, 13..17);
        assert_eq!(lines[1].colors.len(), 2);
        assert_eq!(at_offset(&lines, 15).unwrap().range, 13..17);
        assert_eq!(at_offset(&lines, 17).unwrap().range, 13..17, "the end edge counts");
        assert!(at_offset(&lines, 2).is_none());
    }

    #[test]
    fn hex_keeps_its_length_and_case() {
        let red = [1., 0., 0., 1.];
        let teal = [0.259, 0.643, 0.812, 1.];
        assert_eq!(format(red, &only("#ABC").format), "#F00");
        assert_eq!(format(teal, &only("#abc").format), "#42a4cf", "too fine for 3 digits");
        assert_eq!(format(teal, &only("#123456").format), "#42a4cf");
        assert_eq!(format([1., 0., 0., 0.5], &only("#123456").format), "#ff000080", "alpha added");
        assert_eq!(format(red, &only("#11223344").format), "#ff0000ff", "alpha kept");
    }

    #[test]
    fn functions_keep_their_style() {
        let red = [1., 0., 0., 1.];
        let half = [1., 0., 0., 0.5];
        assert_eq!(format(red, &only("rgb(1, 2, 3)").format), "rgb(255, 0, 0)");
        assert_eq!(format(half, &only("rgb(1, 2, 3)").format), "rgba(255, 0, 0, 0.5)");
        assert_eq!(format(half, &only("rgb(1 2 3)").format), "rgb(255 0 0 / 0.5)");
        assert_eq!(format(red, &only("rgba(1,2,3,0.2)").format), "rgba(255, 0, 0, 1)");
        assert_eq!(format(red, &only("hsl(10 20% 30%)").format), "hsl(0 100% 50%)");
        assert_eq!(format([0., 1., 0., 1.], &only("HSL(1, 2%, 3%)").format), "HSL(120, 100%, 50%)");
    }

    #[test]
    fn round_trips_through_hsl() {
        for rgb in [[0.2, 0.4, 0.6], [1., 1., 1.], [0.9, 0.1, 0.5]] {
            let [h, s, l] = rgb_to_hsl(rgb);
            let back = hsl_to_rgb(h, s, l);
            assert!(rgb.iter().zip(back).all(|(a, b)| (a - b).abs() < 0.001), "{rgb:?} -> {back:?}");
        }
    }
}
