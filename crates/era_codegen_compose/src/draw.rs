//! `EraDraw.kt`: drawing helpers for CSS borders and outlines that Compose
//! has no modifier for (M8 phase 3, D-087): dashed/dotted lines, per-side
//! borders and `outline` with `outline-offset`; CSS gradients, `box-shadow` and colour
//! filters (phase 4). Shadows blur with the host's `eraBlurPaint`.

use crate::GENERATED_HEADER;

pub fn generate(package: &str) -> String {
    format!("{GENERATED_HEADER}\npackage {package}\n{BODY}")
}

const BODY: &str = r#"
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.ClipOp
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.Paint
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathOperation
import androidx.compose.ui.graphics.addOutline
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.LinearGradientShader
import androidx.compose.ui.graphics.RadialGradientShader
import androidx.compose.ui.graphics.Shader
import androidx.compose.ui.graphics.ShaderBrush
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.sin
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawOutline
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection

/** CSS `linear-gradient`: the angle (0 = up, 90 = right), or a corner (`to top right`). */
private data class EraLinear(
    val angle: Float?,
    val right: Boolean,
    val bottom: Boolean,
    val colors: List<Color>,
    val stops: List<Float>,
) : ShaderBrush() {
    override fun createShader(size: Size): Shader {
        val deg = angle ?: run {
            // CSS "magic corners": the line is perpendicular to the other diagonal.
            val a = atan2(size.height, size.width) * 180f / PI.toFloat()
            when {
                right && !bottom -> a
                right && bottom -> 180f - a
                !right && bottom -> 180f + a
                else -> 360f - a
            }
        }
        val rad = deg * PI.toFloat() / 180f
        val s = sin(rad)
        val c = cos(rad)
        val half = (abs(size.width * s) + abs(size.height * c)) / 2f
        val cx = size.width / 2f
        val cy = size.height / 2f
        return LinearGradientShader(
            Offset(cx - s * half, cy + c * half),
            Offset(cx + s * half, cy - c * half),
            colors,
            stops,
        )
    }
}

/** CSS `radial-gradient(circle, ...)` from the centre to the farthest corner. */
private data class EraRadial(val colors: List<Color>, val stops: List<Float>) : ShaderBrush() {
    override fun createShader(size: Size): Shader = RadialGradientShader(
        Offset(size.width / 2f, size.height / 2f),
        hypot(size.width / 2f, size.height / 2f).coerceAtLeast(0.01f),
        colors,
        stops,
    )
}

fun eraLinearGradient(angle: Float, colors: List<Color>, stops: List<Float>): Brush =
    EraLinear(angle, false, false, colors, stops)

fun eraCornerGradient(right: Boolean, bottom: Boolean, colors: List<Color>, stops: List<Float>): Brush =
    EraLinear(null, right, bottom, colors, stops)

fun eraRadialGradient(colors: List<Color>, stops: List<Float>): Brush = EraRadial(colors, stops)

/** One CSS `box-shadow` layer. */
data class EraShadow(val x: Dp, val y: Dp, val blur: Dp, val spread: Dp, val color: Color)

/**
 * CSS `box-shadow` (outer): blurred copies of the shape, never drawn under
 * the element itself, as in CSS. The first shadow is on top.
 */
fun Modifier.eraBoxShadow(shadows: List<EraShadow>, shape: Shape): Modifier = drawWithContent {
    val box = Path().apply { addOutline(shape.createOutline(size, layoutDirection, this@drawWithContent)) }
    clipPath(box, ClipOp.Difference) {
        for (s in shadows.asReversed()) {
            val spread = s.spread.toPx()
            val w = size.width + 2f * spread
            val h = size.height + 2f * spread
            if (w <= 0f || h <= 0f) continue
            val outline = shape.createOutline(Size(w, h), layoutDirection, this)
            val paint = Paint().apply { color = s.color }
            eraBlurPaint(paint, s.blur.toPx() / 2f)
            drawIntoCanvas { canvas ->
                canvas.save()
                canvas.translate(s.x.toPx() - spread, s.y.toPx() - spread)
                canvas.drawOutline(outline, paint)
                canvas.restore()
            }
        }
    }
    drawContent()
}

/** CSS `box-shadow: inset ...`: drawn inside the shape, over the background. */
fun Modifier.eraInsetShadow(shadows: List<EraShadow>, shape: Shape): Modifier = drawWithContent {
    val box = Path().apply { addOutline(shape.createOutline(size, layoutDirection, this@drawWithContent)) }
    clipPath(box) {
        for (s in shadows.asReversed()) {
            val spread = s.spread.toPx()
            val blur = s.blur.toPx()
            val hole = Path()
            val w = size.width - 2f * spread
            val h = size.height - 2f * spread
            if (w > 0f && h > 0f) hole.addOutline(shape.createOutline(Size(w, h), layoutDirection, this))
            hole.translate(Offset(s.x.toPx() + spread, s.y.toPx() + spread))
            val m = blur + abs(s.x.toPx()) + abs(s.y.toPx()) + abs(spread) + 1f
            val frame = Path().apply { addRect(Rect(-m, -m, size.width + m, size.height + m)) }
            val ring = Path().apply { op(frame, hole, PathOperation.Difference) }
            val paint = Paint().apply { color = s.color }
            eraBlurPaint(paint, blur / 2f)
            drawIntoCanvas { it.drawPath(ring, paint) }
        }
    }
    drawContent()
}

/** CSS colour filters (`grayscale`, `brightness`, ...) as one 4x5 colour matrix. */
fun Modifier.eraColorFilter(matrix: FloatArray): Modifier = drawWithContent {
    val paint = Paint().apply { colorFilter = ColorFilter.colorMatrix(ColorMatrix(matrix)) }
    drawIntoCanvas { it.saveLayer(Rect(Offset.Zero, size), paint) }
    drawContent()
    drawIntoCanvas { it.restore() }
}

/** CSS `border-style`. */
enum class EraLine { Solid, Dashed, Dotted }

/** One side of a CSS border. */
data class EraSide(val width: Dp, val color: Color, val line: EraLine = EraLine.Solid)

private fun eraStroke(width: Float, line: EraLine): Stroke = when (line) {
    EraLine.Solid -> Stroke(width)
    EraLine.Dashed -> Stroke(width, pathEffect = PathEffect.dashPathEffect(floatArrayOf(width * 3f, width * 3f)))
    EraLine.Dotted -> Stroke(width, cap = StrokeCap.Round, pathEffect = PathEffect.dashPathEffect(floatArrayOf(0.01f, width * 2f)))
}

private fun DrawScope.eraStrokeShape(shape: Shape, color: Color, width: Float, line: EraLine, grow: Float) {
    val inset = width / 2f - grow
    val s = Size(size.width - 2f * inset, size.height - 2f * inset)
    if (s.width <= 0f || s.height <= 0f) return
    translate(inset, inset) {
        drawOutline(shape.createOutline(s, layoutDirection, this), color, style = eraStroke(width, line))
    }
}

/** A border on every side, following the element's shape (`border: 2px dashed red`). */
fun Modifier.eraBorder(width: Dp, color: Color, shape: Shape, line: EraLine): Modifier = drawWithContent {
    drawContent()
    val w = width.toPx()
    if (w > 0f) eraStrokeShape(shape, color, w, line, 0f)
}

/** Different borders per side (`border-bottom: 1px dashed #ccc`). Corners stay square. */
fun Modifier.eraSideBorders(top: EraSide?, end: EraSide?, bottom: EraSide?, start: EraSide?): Modifier = drawWithContent {
    drawContent()
    val rtl = layoutDirection == LayoutDirection.Rtl
    fun edge(side: EraSide?, from: (Float) -> Offset, to: (Float) -> Offset) {
        if (side == null) return
        val w = side.width.toPx()
        if (w <= 0f) return
        val stroke = eraStroke(w, side.line)
        drawLine(side.color, from(w / 2f), to(w / 2f), strokeWidth = w, cap = stroke.cap, pathEffect = stroke.pathEffect)
    }
    edge(top, { Offset(0f, it) }, { Offset(size.width, it) })
    edge(bottom, { Offset(0f, size.height - it) }, { Offset(size.width, size.height - it) })
    edge(if (rtl) end else start, { Offset(it, 0f) }, { Offset(it, size.height) })
    edge(if (rtl) start else end, { Offset(size.width - it, 0f) }, { Offset(size.width - it, size.height) })
}

/** CSS `outline`: drawn outside the element, takes no space. */
fun Modifier.eraOutline(width: Dp, color: Color, shape: Shape, line: EraLine, offset: Dp): Modifier = drawWithContent {
    drawContent()
    val w = width.toPx()
    if (w > 0f) eraStrokeShape(shape, color, w, line, offset.toPx() + w)
}
"#;

/// The 4x5 colour matrix (Compose `ColorMatrix`, offsets in 0-255) for CSS
/// colour filters applied in order, per the Filter Effects spec. `None`
/// when no colour filter is present.
pub(crate) fn color_matrix(filters: &[(&str, f64)]) -> Option<[f64; 20]> {
    type M = [[f64; 5]; 5];
    let identity = || -> M {
        let mut m = [[0.0; 5]; 5];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        m
    };
    let rgb = |a: [[f64; 3]; 3]| -> M {
        let mut m = identity();
        for i in 0..3 {
            m[i][..3].copy_from_slice(&a[i]);
        }
        m
    };
    let mut total = identity();
    let mut any = false;
    for (name, a) in filters {
        let a = *a;
        let f: M = match *name {
            "brightness" => rgb([[a, 0.0, 0.0], [0.0, a, 0.0], [0.0, 0.0, a]]),
            "contrast" => {
                let mut m = rgb([[a, 0.0, 0.0], [0.0, a, 0.0], [0.0, 0.0, a]]);
                for row in m.iter_mut().take(3) {
                    row[4] = (0.5 - 0.5 * a) * 255.0;
                }
                m
            }
            "grayscale" => {
                let b = 1.0 - a;
                rgb([
                    [
                        0.2126 + 0.7874 * b,
                        0.7152 - 0.7152 * b,
                        0.0722 - 0.0722 * b,
                    ],
                    [
                        0.2126 - 0.2126 * b,
                        0.7152 + 0.2848 * b,
                        0.0722 - 0.0722 * b,
                    ],
                    [
                        0.2126 - 0.2126 * b,
                        0.7152 - 0.7152 * b,
                        0.0722 + 0.9278 * b,
                    ],
                ])
            }
            "sepia" => {
                let b = 1.0 - a;
                rgb([
                    [0.393 + 0.607 * b, 0.769 - 0.769 * b, 0.189 - 0.189 * b],
                    [0.349 - 0.349 * b, 0.686 + 0.314 * b, 0.168 - 0.168 * b],
                    [0.272 - 0.272 * b, 0.534 - 0.534 * b, 0.131 + 0.869 * b],
                ])
            }
            "saturate" => rgb([
                [0.213 + 0.787 * a, 0.715 - 0.715 * a, 0.072 - 0.072 * a],
                [0.213 - 0.213 * a, 0.715 + 0.285 * a, 0.072 - 0.072 * a],
                [0.213 - 0.213 * a, 0.715 - 0.715 * a, 0.072 + 0.928 * a],
            ]),
            "hue-rotate" => {
                let (s, c) = a.to_radians().sin_cos();
                rgb([
                    [
                        0.213 + c * 0.787 - s * 0.213,
                        0.715 - c * 0.715 - s * 0.715,
                        0.072 - c * 0.072 + s * 0.928,
                    ],
                    [
                        0.213 - c * 0.213 + s * 0.143,
                        0.715 + c * 0.285 + s * 0.140,
                        0.072 - c * 0.072 - s * 0.283,
                    ],
                    [
                        0.213 - c * 0.213 - s * 0.787,
                        0.715 - c * 0.715 + s * 0.715,
                        0.072 + c * 0.928 + s * 0.072,
                    ],
                ])
            }
            "invert" => {
                let mut m = rgb([
                    [1.0 - 2.0 * a, 0.0, 0.0],
                    [0.0, 1.0 - 2.0 * a, 0.0],
                    [0.0, 0.0, 1.0 - 2.0 * a],
                ]);
                for row in m.iter_mut().take(3) {
                    row[4] = a * 255.0;
                }
                m
            }
            "opacity" => {
                let mut m = identity();
                m[3][3] = a;
                m
            }
            _ => continue,
        };
        any = true;
        // Apply `f` after everything so far: total = f * total.
        let mut next = [[0.0; 5]; 5];
        for (i, row) in next.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..5).map(|k| f[i][k] * total[k][j]).sum();
            }
        }
        total = next;
    }
    if !any {
        return None;
    }
    let mut out = [0.0; 20];
    for i in 0..4 {
        out[i * 5..i * 5 + 5].copy_from_slice(&total[i]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_matrices() {
        assert_eq!(color_matrix(&[("blur", 4.0)]), None);
        let g = color_matrix(&[("grayscale", 1.0)]).unwrap();
        assert!((g[0] - 0.2126).abs() < 1e-9 && (g[1] - 0.7152).abs() < 1e-9);
        // brightness(2) then brightness(.5) is the identity.
        let m = color_matrix(&[("brightness", 2.0), ("brightness", 0.5)]).unwrap();
        assert!((m[0] - 1.0).abs() < 1e-9 && (m[6] - 1.0).abs() < 1e-9);
        let inv = color_matrix(&[("invert", 1.0)]).unwrap();
        assert_eq!((inv[0], inv[4]), (-1.0, 255.0));
        // hue-rotate(0) is the identity.
        let h = color_matrix(&[("hue-rotate", 0.0)]).unwrap();
        assert!((h[0] - 1.0).abs() < 1e-9 && h[1].abs() < 1e-9);
    }
}
