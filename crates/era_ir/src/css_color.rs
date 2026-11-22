//! CSS colour syntaxes (M8 phase 4, D-088): the 148 named colours,
//! `transparent`, `rgb()`/`rgba()` and `hsl()`/`hsla()` in both the comma
//! and the space syntax (`rgb(0 0 0 / 50%)`).

use era_syntax::{Color, Value as AstValue, ValueKind};

/// CSS named colours (CSS Color Module Level 4), as `0xRRGGBB`.
const NAMED: &[(&str, u32)] = &[
    ("aliceblue", 0xF0F8FF),
    ("antiquewhite", 0xFAEBD7),
    ("aqua", 0x00FFFF),
    ("aquamarine", 0x7FFFD4),
    ("azure", 0xF0FFFF),
    ("beige", 0xF5F5DC),
    ("bisque", 0xFFE4C4),
    ("black", 0x000000),
    ("blanchedalmond", 0xFFEBCD),
    ("blue", 0x0000FF),
    ("blueviolet", 0x8A2BE2),
    ("brown", 0xA52A2A),
    ("burlywood", 0xDEB887),
    ("cadetblue", 0x5F9EA0),
    ("chartreuse", 0x7FFF00),
    ("chocolate", 0xD2691E),
    ("coral", 0xFF7F50),
    ("cornflowerblue", 0x6495ED),
    ("cornsilk", 0xFFF8DC),
    ("crimson", 0xDC143C),
    ("cyan", 0x00FFFF),
    ("darkblue", 0x00008B),
    ("darkcyan", 0x008B8B),
    ("darkgoldenrod", 0xB8860B),
    ("darkgray", 0xA9A9A9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xA9A9A9),
    ("darkkhaki", 0xBDB76B),
    ("darkmagenta", 0x8B008B),
    ("darkolivegreen", 0x556B2F),
    ("darkorange", 0xFF8C00),
    ("darkorchid", 0x9932CC),
    ("darkred", 0x8B0000),
    ("darksalmon", 0xE9967A),
    ("darkseagreen", 0x8FBC8F),
    ("darkslateblue", 0x483D8B),
    ("darkslategray", 0x2F4F4F),
    ("darkslategrey", 0x2F4F4F),
    ("darkturquoise", 0x00CED1),
    ("darkviolet", 0x9400D3),
    ("deeppink", 0xFF1493),
    ("deepskyblue", 0x00BFFF),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1E90FF),
    ("firebrick", 0xB22222),
    ("floralwhite", 0xFFFAF0),
    ("forestgreen", 0x228B22),
    ("fuchsia", 0xFF00FF),
    ("gainsboro", 0xDCDCDC),
    ("ghostwhite", 0xF8F8FF),
    ("gold", 0xFFD700),
    ("goldenrod", 0xDAA520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xADFF2F),
    ("grey", 0x808080),
    ("honeydew", 0xF0FFF0),
    ("hotpink", 0xFF69B4),
    ("indianred", 0xCD5C5C),
    ("indigo", 0x4B0082),
    ("ivory", 0xFFFFF0),
    ("khaki", 0xF0E68C),
    ("lavender", 0xE6E6FA),
    ("lavenderblush", 0xFFF0F5),
    ("lawngreen", 0x7CFC00),
    ("lemonchiffon", 0xFFFACD),
    ("lightblue", 0xADD8E6),
    ("lightcoral", 0xF08080),
    ("lightcyan", 0xE0FFFF),
    ("lightgoldenrodyellow", 0xFAFAD2),
    ("lightgray", 0xD3D3D3),
    ("lightgreen", 0x90EE90),
    ("lightgrey", 0xD3D3D3),
    ("lightpink", 0xFFB6C1),
    ("lightsalmon", 0xFFA07A),
    ("lightseagreen", 0x20B2AA),
    ("lightskyblue", 0x87CEFA),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xB0C4DE),
    ("lightyellow", 0xFFFFE0),
    ("lime", 0x00FF00),
    ("limegreen", 0x32CD32),
    ("linen", 0xFAF0E6),
    ("magenta", 0xFF00FF),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66CDAA),
    ("mediumblue", 0x0000CD),
    ("mediumorchid", 0xBA55D3),
    ("mediumpurple", 0x9370DB),
    ("mediumseagreen", 0x3CB371),
    ("mediumslateblue", 0x7B68EE),
    ("mediumspringgreen", 0x00FA9A),
    ("mediumturquoise", 0x48D1CC),
    ("mediumvioletred", 0xC71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xF5FFFA),
    ("mistyrose", 0xFFE4E1),
    ("moccasin", 0xFFE4B5),
    ("navajowhite", 0xFFDEAD),
    ("navy", 0x000080),
    ("oldlace", 0xFDF5E6),
    ("olive", 0x808000),
    ("olivedrab", 0x6B8E23),
    ("orange", 0xFFA500),
    ("orangered", 0xFF4500),
    ("orchid", 0xDA70D6),
    ("palegoldenrod", 0xEEE8AA),
    ("palegreen", 0x98FB98),
    ("paleturquoise", 0xAFEEEE),
    ("palevioletred", 0xDB7093),
    ("papayawhip", 0xFFEFD5),
    ("peachpuff", 0xFFDAB9),
    ("peru", 0xCD853F),
    ("pink", 0xFFC0CB),
    ("plum", 0xDDA0DD),
    ("powderblue", 0xB0E0E6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xFF0000),
    ("rosybrown", 0xBC8F8F),
    ("royalblue", 0x4169E1),
    ("saddlebrown", 0x8B4513),
    ("salmon", 0xFA8072),
    ("sandybrown", 0xF4A460),
    ("seagreen", 0x2E8B57),
    ("seashell", 0xFFF5EE),
    ("sienna", 0xA0522D),
    ("silver", 0xC0C0C0),
    ("skyblue", 0x87CEEB),
    ("slateblue", 0x6A5ACD),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xFFFAFA),
    ("springgreen", 0x00FF7F),
    ("steelblue", 0x4682B4),
    ("tan", 0xD2B48C),
    ("teal", 0x008080),
    ("thistle", 0xD8BFD8),
    ("tomato", 0xFF6347),
    ("turquoise", 0x40E0D0),
    ("violet", 0xEE82EE),
    ("wheat", 0xF5DEB3),
    ("white", 0xFFFFFF),
    ("whitesmoke", 0xF5F5F5),
    ("yellow", 0xFFFF00),
    ("yellowgreen", 0x9ACD32),
];

/// A named CSS colour or `transparent` (case-insensitive, like CSS).
pub fn named(name: &str) -> Option<Color> {
    let n = name.to_ascii_lowercase();
    if n == "transparent" {
        return Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    NAMED.iter().find(|(k, _)| *k == n).map(|(_, v)| Color {
        r: (v >> 16) as u8,
        g: (v >> 8) as u8,
        b: *v as u8,
        a: 255,
    })
}

pub fn named_count() -> usize {
    NAMED.len()
}

/// `rgb()`, `rgba()`, `hsl()` or `hsla()`. `Ok(None)` if `name` is not a
/// colour function.
pub fn function(name: &str, args: &[AstValue]) -> Result<Option<Color>, String> {
    let name = name.to_ascii_lowercase();
    let hsl = match name.as_str() {
        "rgb" | "rgba" => false,
        "hsl" | "hsla" => true,
        _ => return Ok(None),
    };
    // Flatten `rgb(1, 2, 3)` and `rgb(1 2 3 / 50%)` into channels + alpha.
    let mut channels: Vec<&AstValue> = Vec::new();
    let mut alpha: Option<&AstValue> = None;
    let mut after_slash = false;
    for a in args {
        let parts: Vec<&AstValue> = match &a.kind {
            ValueKind::Terms(t) => t.iter().collect(),
            _ => vec![a],
        };
        for p in parts {
            match &p.kind {
                ValueKind::Slash => after_slash = true,
                _ if after_slash => alpha = Some(p),
                _ => channels.push(p),
            }
        }
    }
    if channels.len() == 4 && alpha.is_none() {
        alpha = channels.pop();
    }
    let usage = if hsl {
        "use `hsl(210 50% 40%)` or `hsla(210, 50%, 40%, .5)`"
    } else {
        "use `rgb(20 40 60)`, `rgb(20 40 60 / 50%)` or `rgba(20, 40, 60, .5)`"
    };
    if channels.len() != 3 {
        return Err(format!("`{name}()` needs 3 channels; {usage}"));
    }
    let num = |v: &AstValue| -> Result<(f64, Option<String>), String> {
        match &v.kind {
            ValueKind::Number(n) => Ok((*n, None)),
            ValueKind::Dimension(n, u) => Ok((*n, Some(u.clone()))),
            _ => Err(format!("`{name}()` takes numbers; {usage}")),
        }
    };
    let a = match alpha {
        None => 1.0,
        Some(v) => match num(v)? {
            (n, None) => n,
            (n, Some(u)) if u == "%" => n / 100.0,
            _ => return Err(format!("the alpha of `{name}()` is a number or a %")),
        },
    }
    .clamp(0.0, 1.0);
    let byte = |x: f64| (x.clamp(0.0, 255.0)).round() as u8;
    let (r, g, b) = if hsl {
        let h = match num(channels[0])? {
            (n, None) => n,
            (n, Some(u)) => match u.as_str() {
                "deg" => n,
                "turn" => n * 360.0,
                "rad" => n.to_degrees(),
                _ => return Err(format!("the hue of `{name}()` is a number or an angle")),
            },
        };
        let pct = |v: &AstValue| -> Result<f64, String> {
            match num(v)? {
                (n, Some(u)) if u == "%" => Ok((n / 100.0).clamp(0.0, 1.0)),
                (n, None) => Ok((n / 100.0).clamp(0.0, 1.0)),
                _ => Err(format!("saturation and lightness are percentages; {usage}")),
            }
        };
        let (s, l) = (pct(channels[1])?, pct(channels[2])?);
        let (r, g, b) = hsl_to_rgb(h.rem_euclid(360.0), s, l);
        (byte(r * 255.0), byte(g * 255.0), byte(b * 255.0))
    } else {
        let ch = |v: &AstValue| -> Result<u8, String> {
            match num(v)? {
                (n, None) => Ok(byte(n)),
                (n, Some(u)) if u == "%" => Ok(byte(n * 2.55)),
                _ => Err(format!("`{name}()` channels are 0-255 or a %; {usage}")),
            }
        };
        (ch(channels[0])?, ch(channels[1])?, ch(channels[2])?)
    };
    Ok(Some(Color {
        r,
        g,
        b,
        a: byte(a * 255.0),
    }))
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_every_named_colour() {
        // 148 names in CSS Color 4 (including grey/gray spellings), plus `transparent`.
        assert_eq!(named_count(), 148);
        assert_eq!(
            named("RebeccaPurple"),
            Some(Color {
                r: 0x66,
                g: 0x33,
                b: 0x99,
                a: 255
            })
        );
        assert_eq!(named("transparent").unwrap().a, 0);
        assert_eq!(named("notacolour"), None);
    }

    #[test]
    fn hsl_converts() {
        let (r, g, b) = hsl_to_rgb(120.0, 1.0, 0.25);
        assert_eq!(
            (
                (r * 255.0).round(),
                (g * 255.0).round(),
                (b * 255.0).round()
            ),
            (0.0, 128.0, 0.0)
        );
    }
}
