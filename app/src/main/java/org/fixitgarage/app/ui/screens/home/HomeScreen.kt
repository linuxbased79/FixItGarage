package org.fixitgarage.app.ui.screens.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.data.entity.ServiceRecordEntity
import org.fixitgarage.app.data.entity.VehicleEntity
import org.fixitgarage.app.domain.model.UserMode
import org.fixitgarage.app.ui.components.EmptyState
import org.fixitgarage.app.ui.components.SectionCard
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
fun HomeScreen(
    vehicles: List<VehicleEntity>,
    lastService: ServiceRecordEntity?,
    userMode: UserMode,
    onAddVehicle: () -> Unit,
    onScanReceipt: () -> Unit,
    onOpenTires: () -> Unit,
    modifier: Modifier = Modifier
) {
    val dateFmt = SimpleDateFormat("MMM d, yyyy", Locale.getDefault())

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("FixItGarage", style = MaterialTheme.typography.headlineMedium)
        Text(
            when (userMode) {
                UserMode.DIY -> "DIY-focused tools ready"
                UserMode.SHOP -> "Shop & receipt tools ready"
                UserMode.BOTH -> "Full garage toolkit"
            },
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        SectionCard(
            title = "Last service",
            subtitle = "Quick view from your maintenance history"
        ) {
            if (lastService == null) {
                EmptyState("No services logged yet. Scan a receipt or add one manually.")
            } else {
                Text(lastService.title, style = MaterialTheme.typography.titleSmall)
                Text(
                    "${dateFmt.format(Date(lastService.dateEpochMs))} · ${lastService.mileage} mi · ${lastService.source}",
                    style = MaterialTheme.typography.bodyMedium
                )
                val total = lastService.laborCost + lastService.partsCost + (lastService.fuelCost ?: 0.0)
                if (total > 0) {
                    Text(
                        "Cost: $${"%.2f".format(total)}",
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }
        }

        SectionCard(title = "Vehicles", subtitle = "Unlimited vehicles supported") {
            if (vehicles.isEmpty()) {
                EmptyState("Add your first vehicle to get started.")
                Button(onClick = onAddVehicle) { Text("Add vehicle") }
            } else {
                vehicles.take(5).forEach { v ->
                    Text(
                        "${v.name} — ${v.currentMileage} mi",
                        style = MaterialTheme.typography.bodyLarge
                    )
                }
                if (vehicles.size > 5) {
                    Text("+ ${vehicles.size - 5} more", style = MaterialTheme.typography.bodySmall)
                }
            }
        }

        SectionCard(title = "Quick actions") {
            Button(onClick = onScanReceipt) { Text("Scan receipt (OCR)") }
            Button(onClick = onOpenTires) { Text("Tire rotation diagram") }
        }
    }
}
