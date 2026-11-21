//! `EraRuntime.kt`: small shared helpers the generated screens call.

use crate::GENERATED_HEADER;

pub fn generate(package: &str) -> String {
    format!(
        "{GENERATED_HEADER}
package {package}

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle

/** A screen the host can show. */
class EraScreen(val name: String, val title: String?, val content: @Composable () -> Unit)

/** Navigation stack (M6). `go` pushes a screen, `back` pops one. */
object EraNav {{
    val stack = mutableStateListOf<String>()
    val current: String? get() = stack.lastOrNull()
    val canGoBack: Boolean get() = stack.size > 1
    fun reset(screen: String) {{ stack.clear(); stack.add(screen) }}
    fun go(screen: String) {{ stack.add(screen) }}
    fun back() {{ if (stack.size > 1) stack.removeAt(stack.lastIndex) }}
}}

/** Shows the screen on top of the navigation stack (the start screen at first). */
@Composable
fun EraApp() {{
    val name = EraNav.current ?: EraStartScreen
    val screen = EraScreens.firstOrNull {{ it.name == name }} ?: EraScreens.first()
    screen.content()
}}

/** Numbers in text: `3.0` shows as `3`. */
fun eraText(x: Double): String =
    if (x % 1.0 == 0.0 && kotlin.math.abs(x) < 1e15) x.toLong().toString() else x.toString()

/** Text input with a placeholder. Bound to state with `bind`, otherwise it keeps its own text. */
@Composable
fun EraInput(
    placeholder: String,
    style: TextStyle,
    placeholderColor: Color,
    modifier: Modifier,
    value: String? = null,
    onValueChange: ((String) -> Unit)? = null,
) {{
    var local by remember {{ mutableStateOf(\"\") }}
    val text = value ?: local
    BasicTextField(
        value = text,
        onValueChange = {{ if (onValueChange != null) onValueChange(it) else local = it }},
        modifier = modifier,
        textStyle = style,
        singleLine = true,
        decorationBox = {{ inner ->
            Box {{
                if (text.isEmpty()) BasicText(placeholder, style = style.copy(color = placeholderColor))
                inner()
            }}
        }},
    )
}}
"
    )
}
