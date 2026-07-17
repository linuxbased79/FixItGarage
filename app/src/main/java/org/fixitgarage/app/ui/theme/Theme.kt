package org.fixitgarage.app.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

private val GreenPrimary = Color(0xFF2E7D32)
private val GreenOnPrimary = Color(0xFFFFFFFF)
private val GreenContainer = Color(0xFFC8E6C9)
private val GreenOnContainer = Color(0xFF1B5E20)

private val LightColors = lightColorScheme(
    primary = GreenPrimary,
    onPrimary = GreenOnPrimary,
    primaryContainer = GreenContainer,
    onPrimaryContainer = GreenOnContainer,
    secondary = Color(0xFF546E7A),
    tertiary = Color(0xFF6D4C41)
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFF81C784),
    onPrimary = Color(0xFF003910),
    primaryContainer = Color(0xFF1B5E20),
    onPrimaryContainer = Color(0xFFC8E6C9),
    secondary = Color(0xFFB0BEC5),
    tertiary = Color(0xFFBCAAA4)
)

/**
 * Full dark-mode support via system, light, or dark preference.
 * Dynamic color is optional and only used when available (not required for GrapheneOS).
 */
@Composable
fun FixItGarageTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColors
        else -> LightColors
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
