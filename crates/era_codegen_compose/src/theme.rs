//! `EraTheme.kt`: the skin as Kotlin objects, plus light/dark selection.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use era_ir::ir::{App, Color};

use crate::GENERATED_HEADER;
use crate::kotlin::{color, dp, ident, sp, string};

pub fn generate(app: &App, package: &str) -> String {
    let skin = &app.skin;
    let mut o = String::new();
    o.push_str(GENERATED_HEADER);
    let _ = writeln!(
        o,
        "// Skin: {}{}\n",
        skin.name,
        if skin.builtin {
            " (built-in default)"
        } else {
            ""
        }
    );
    let _ = writeln!(o, "package {package}\n");
    o.push_str(
        "import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

",
    );

    // Colors: one class, a light instance and (optionally) a dark one.
    o.push_str("@Immutable\nclass EraColors(\n");
    for k in skin.colors.keys() {
        let _ = writeln!(o, "    val {}: Color,", ident(k));
    }
    o.push_str(")\n\n");
    palette(&mut o, "EraLightColors", &skin.colors);
    match &skin.dark_colors {
        Some(d) => palette(&mut o, "EraDarkColors", d),
        None => o.push_str("/** This skin has no `colors dark`, so dark mode uses the light colors. */\nval EraDarkColors: EraColors = EraLightColors\n\n"),
    }
    let _ = writeln!(
        o,
        "const val EraHasDarkMode: Boolean = {}\n",
        skin.dark_colors.is_some()
    );

    for (name, table) in [
        ("EraSpacing", &skin.spacing),
        ("EraRadius", &skin.radius),
        ("EraBorder", &skin.border),
        ("EraElevation", &skin.elevation),
    ] {
        let _ = writeln!(o, "object {name} {{");
        for (k, v) in table {
            let _ = writeln!(o, "    val {} = {}", ident(k), dp(*v));
        }
        o.push_str("}\n\n");
    }

    // Typography.
    let t = &skin.typography;
    if t.font_files.is_empty() {
        let _ = writeln!(
            o,
            "/** Font: {}. */\nval EraFont: FontFamily = FontFamily.Default\n",
            t.font
        );
    } else {
        let files: Vec<String> = t
            .font_files
            .iter()
            .map(|f| string(f.trim_start_matches("Assets/")))
            .collect();
        let _ = writeln!(
            o,
            "/** Font: {}. Loaded by the platform host. */\nval EraFont: FontFamily by lazy {{ eraFontFamily(listOf({})) }}\n",
            t.font,
            files.join(", ")
        );
    }
    o.push_str("object EraType {\n");
    for (k, s) in &t.styles {
        let mut args = vec![
            "fontFamily = EraFont".to_string(),
            format!("fontSize = {}", sp(s.size)),
            format!("fontWeight = FontWeight({})", s.weight),
        ];
        if let Some(lh) = s.line_height {
            args.push(format!("lineHeight = {}", sp(lh)));
        }
        if let Some(tr) = s.tracking {
            args.push(format!("letterSpacing = {}", sp(tr)));
        }
        let _ = writeln!(o, "    val {} = TextStyle({})", ident(k), args.join(", "));
    }
    o.push_str("}\n\n");

    o.push_str(
        "val LocalEraColors = staticCompositionLocalOf { EraLightColors }

/** Provides the skin's colors; `dark` selects `colors dark` when the skin has it. */
@Composable
fun EraTheme(dark: Boolean = false, content: @Composable () -> Unit) {
    CompositionLocalProvider(
        LocalEraColors provides if (dark) EraDarkColors else EraLightColors,
        content = content,
    )
}
",
    );
    o
}

fn palette(o: &mut String, name: &str, colors: &BTreeMap<String, Color>) {
    let _ = writeln!(o, "val {name} = EraColors(");
    for (k, c) in colors {
        let _ = writeln!(o, "    {} = {},", ident(k), color(c));
    }
    o.push_str(")\n\n");
}
