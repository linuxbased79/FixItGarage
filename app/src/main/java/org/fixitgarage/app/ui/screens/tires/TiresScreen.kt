package org.fixitgarage.app.ui.screens.tires

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.ui.components.SectionCard

enum class RotationPattern(val label: String) {
    FORWARD_CROSS("Forward cross"),
    REARWARD_CROSS("Rearward cross"),
    X_PATTERN("X pattern"),
    SIDE_TO_SIDE("Side to side")
}

/** Tire labels at FL, FR, RL, RR */
data class TireLayout(
    val fl: String = "FL",
    val fr: String = "FR",
    val rl: String = "RL",
    val rr: String = "RR"
)

fun applyRotation(current: TireLayout, pattern: RotationPattern): TireLayout = when (pattern) {
    RotationPattern.FORWARD_CROSS -> TireLayout(
        fl = current.rl,
        fr = current.rr,
        rl = current.fr,
        rr = current.fl
    )
    RotationPattern.REARWARD_CROSS -> TireLayout(
        fl = current.rr,
        fr = current.rl,
        rl = current.fl,
        rr = current.fr
    )
    RotationPattern.X_PATTERN -> TireLayout(
        fl = current.rr,
        fr = current.rl,
        rl = current.fr,
        rr = current.fl
    )
    RotationPattern.SIDE_TO_SIDE -> TireLayout(
        fl = current.fr,
        fr = current.fl,
        rl = current.rr,
        rr = current.rl
    )
}

@Composable
fun TiresScreen(modifier: Modifier = Modifier) {
    var pattern by remember { mutableStateOf(RotationPattern.FORWARD_CROSS) }
    var layout by remember { mutableStateOf(TireLayout("A", "B", "C", "D")) }
    val preview = remember(layout, pattern) { applyRotation(layout, pattern) }
    var mileageA by remember { mutableStateOf(12000) }
    var mileageB by remember { mutableStateOf(12000) }
    var mileageC by remember { mutableStateOf(11800) }
    var mileageD by remember { mutableStateOf(11800) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Tire tracker", style = MaterialTheme.typography.headlineMedium)
        Text(
            "Top-down diagram, rotation patterns, before/after preview, mileage per tire. " +
                "Receipt scan and camera tread-depth measurement are next.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        SectionCard(title = "Current positions (top-down)") {
            CarTireDiagram(layout = layout, modifier = Modifier.fillMaxWidth())
            Text(
                "Mileage — A: $mileageA · B: $mileageB · C: $mileageC · D: $mileageD",
                style = MaterialTheme.typography.bodySmall
            )
        }

        SectionCard(title = "Rotation pattern") {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                RotationPattern.entries.forEach { p ->
                    FilterChip(
                        selected = pattern == p,
                        onClick = { pattern = p },
                        label = { Text(p.label, style = MaterialTheme.typography.labelSmall) }
                    )
                }
            }
        }

        SectionCard(title = "Before → After preview") {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("Before", style = MaterialTheme.typography.labelLarge)
                    CarTireDiagram(layout = layout, modifier = Modifier.width(150.dp))
                }
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("After", style = MaterialTheme.typography.labelLarge)
                    CarTireDiagram(layout = preview, modifier = Modifier.width(150.dp))
                }
            }
            Button(
                onClick = { layout = preview },
                modifier = Modifier.padding(top = 8.dp)
            ) {
                Text("Apply rotation to log")
            }
        }

        SectionCard(
            title = "Coming next",
            subtitle = "Receipt scan for tire purchases · camera-assisted tread depth"
        ) {
            Text("Data tables for tire sets, positions, and rotation history are already in Room.")
        }
    }
}

@Composable
fun CarTireDiagram(
    layout: TireLayout,
    modifier: Modifier = Modifier
) {
    val outline = MaterialTheme.colorScheme.outline
    val tireColor = MaterialTheme.colorScheme.primary
    val bodyColor = MaterialTheme.colorScheme.surfaceContainerHighest

    Column(modifier = modifier, horizontalAlignment = Alignment.CenterHorizontally) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(0.75f)
                .padding(8.dp)
        ) {
            Canvas(modifier = Modifier.fillMaxSize()) {
                val w = size.width
                val h = size.height
                // Car body
                drawRoundRect(
                    color = bodyColor,
                    topLeft = Offset(w * 0.22f, h * 0.12f),
                    size = Size(w * 0.56f, h * 0.76f),
                    cornerRadius = CornerRadius(w * 0.08f, w * 0.08f)
                )
                drawRoundRect(
                    color = outline,
                    topLeft = Offset(w * 0.22f, h * 0.12f),
                    size = Size(w * 0.56f, h * 0.76f),
                    cornerRadius = CornerRadius(w * 0.08f, w * 0.08f),
                    style = Stroke(width = 3f)
                )
                // Windshield hint
                drawRoundRect(
                    color = outline.copy(alpha = 0.4f),
                    topLeft = Offset(w * 0.30f, h * 0.20f),
                    size = Size(w * 0.40f, h * 0.12f),
                    cornerRadius = CornerRadius(8f, 8f),
                    style = Stroke(width = 2f)
                )
            }

            // Tire labels at corners
            TireLabel(layout.fl, Modifier.align(Alignment.TopStart).padding(4.dp), tireColor)
            TireLabel(layout.fr, Modifier.align(Alignment.TopEnd).padding(4.dp), tireColor)
            TireLabel(layout.rl, Modifier.align(Alignment.BottomStart).padding(4.dp), tireColor)
            TireLabel(layout.rr, Modifier.align(Alignment.BottomEnd).padding(4.dp), tireColor)

            Text(
                "FRONT",
                modifier = Modifier.align(Alignment.TopCenter).padding(top = 2.dp),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Spacer(Modifier.height(4.dp))
    }
}

@Composable
private fun TireLabel(text: String, modifier: Modifier, color: Color) {
    Box(
        modifier = modifier
            .border(2.dp, color, MaterialTheme.shapes.small)
            .padding(horizontal = 10.dp, vertical = 8.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.titleMedium,
            color = color,
            textAlign = TextAlign.Center
        )
    }
}
