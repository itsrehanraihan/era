//! `EraDraw.kt`: drawing helpers for CSS borders and outlines that Compose
//! has no modifier for (M8 phase 3, D-087): dashed/dotted lines, per-side
//! borders and `outline` with `outline-offset`.

use crate::GENERATED_HEADER;

pub fn generate(package: &str) -> String {
    format!("{GENERATED_HEADER}\npackage {package}\n{BODY}")
}

const BODY: &str = r#"
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
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
