// Android implementation of the ERA platform host API.
package {{PACKAGE}}

import android.content.Context
import android.graphics.BitmapFactory
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.painter.BitmapPainter
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight

/** Set by MainActivity before any UI is shown. */
lateinit var EraAppContext: Context

/** Images and fonts are packaged in the APK's `assets/` folder at their ERA paths. */
@Composable
fun eraImage(path: String): Painter {
    val context = LocalContext.current
    return remember(path) {
        val bitmap = context.assets.open(path).use { BitmapFactory.decodeStream(it) }
        BitmapPainter(bitmap.asImageBitmap())
    }
}

/** SVG icons are converted to VectorDrawables at build time (see EraIconIds.kt). */
@Composable
fun eraIcon(path: String): Painter = painterResource(EraIconIds.getValue(path))

fun eraFontFamily(files: List<String>): FontFamily =
    FontFamily(files.map { Font(it, EraAppContext.assets, weightFromName(it)) })

/** `Inter-SemiBold.ttf` -> 600. Unknown names are regular (400). */
private fun weightFromName(path: String): FontWeight {
    val n = path.substringAfterLast('/').lowercase()
    return when {
        "extralight" in n || "ultralight" in n -> FontWeight(200)
        "semibold" in n || "demibold" in n -> FontWeight(600)
        "extrabold" in n || "ultrabold" in n -> FontWeight(800)
        "thin" in n -> FontWeight(100)
        "light" in n -> FontWeight(300)
        "medium" in n -> FontWeight(500)
        "black" in n || "heavy" in n -> FontWeight(900)
        "bold" in n -> FontWeight(700)
        else -> FontWeight(400)
    }
}
