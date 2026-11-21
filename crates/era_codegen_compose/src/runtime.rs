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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle

/** A screen the host can show. */
class EraScreen(val name: String, val title: String?, val content: @Composable () -> Unit)

/** Text input with a placeholder. Holds its own text until ERA has state (M6). */
@Composable
fun EraInput(placeholder: String, style: TextStyle, placeholderColor: Color, modifier: Modifier) {{
    var value by remember {{ mutableStateOf(\"\") }}
    BasicTextField(
        value = value,
        onValueChange = {{ value = it }},
        modifier = modifier,
        textStyle = style,
        singleLine = true,
        decorationBox = {{ inner ->
            Box {{
                if (value.isEmpty()) BasicText(placeholder, style = style.copy(color = placeholderColor))
                inner()
            }}
        }},
    )
}}
"
    )
}
