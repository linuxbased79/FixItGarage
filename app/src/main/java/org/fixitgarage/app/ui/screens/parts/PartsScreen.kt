package org.fixitgarage.app.ui.screens.parts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.ui.components.SectionCard

private val partTypes = listOf(
    "Engine air filter" to "ENGINE_AIR_FILTER",
    "Cabin filter" to "CABIN_FILTER",
    "Oil filter" to "OIL_FILTER",
    "Oil type / viscosity" to "OIL_TYPE"
)

@Composable
fun PartsScreen(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Parts log", style = MaterialTheme.typography.headlineMedium)
        Text(
            "Track brand and part numbers for filters and oil.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        partTypes.forEach { (label, code) ->
            SectionCard(title = label, subtitle = "Type code: $code") {
                Text("No entry yet — UI form wired to Room PartLogEntity next.")
            }
        }
    }
}
