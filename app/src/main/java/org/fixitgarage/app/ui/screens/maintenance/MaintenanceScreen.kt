package org.fixitgarage.app.ui.screens.maintenance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
fun MaintenanceScreen(
    vehicles: List<VehicleEntity>,
    services: List<ServiceRecordEntity>,
    userMode: UserMode,
    onSave: (ServiceRecordEntity) -> Unit,
    onScanReceipt: () -> Unit,
    modifier: Modifier = Modifier
) {
    val dateFmt = SimpleDateFormat("yyyy-MM-dd", Locale.US)
    var title by remember { mutableStateOf("") }
    var mileage by remember { mutableStateOf("") }
    var source by remember {
        mutableStateOf(if (userMode == UserMode.SHOP) "SHOP" else "DIY")
    }
    var cost by remember { mutableStateOf("") }
    var gallons by remember { mutableStateOf("") }

    val vehicleId = vehicles.firstOrNull()?.id

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Maintenance history", style = MaterialTheme.typography.headlineMedium)
        Text(
            "Shop work and DIY oil changes in one timeline.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        Button(onClick = onScanReceipt) { Text("Scan receipt (OCR)") }

        SectionCard(title = "Log service") {
            if (vehicleId == null) {
                EmptyState("Add a vehicle first.")
            } else {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Title (e.g. Oil change)") },
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = mileage,
                    onValueChange = { mileage = it.filter(Char::isDigit) },
                    label = { Text("Mileage") },
                    modifier = Modifier.fillMaxWidth()
                )
                androidx.compose.foundation.layout.Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = source == "DIY",
                        onClick = { source = "DIY" },
                        label = { Text("DIY") }
                    )
                    FilterChip(
                        selected = source == "SHOP",
                        onClick = { source = "SHOP" },
                        label = { Text("Shop") }
                    )
                }
                OutlinedTextField(
                    value = cost,
                    onValueChange = { cost = it },
                    label = { Text("Parts + labor cost") },
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = gallons,
                    onValueChange = { gallons = it },
                    label = { Text("Gallons (fuel fill-up, optional)") },
                    modifier = Modifier.fillMaxWidth()
                )
                Button(
                    onClick = {
                        if (title.isBlank() || vehicleId == null) return@Button
                        val parts = cost.toDoubleOrNull() ?: 0.0
                        onSave(
                            ServiceRecordEntity(
                                vehicleId = vehicleId,
                                dateEpochMs = System.currentTimeMillis(),
                                mileage = mileage.toIntOrNull() ?: 0,
                                title = title.trim(),
                                source = source,
                                partsCost = parts,
                                gallons = gallons.toDoubleOrNull()
                            )
                        )
                        title = ""; mileage = ""; cost = ""; gallons = ""
                    }
                ) { Text("Save") }
            }
        }

        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(services, key = { it.id }) { s ->
                SectionCard(
                    title = s.title,
                    subtitle = "${dateFmt.format(Date(s.dateEpochMs))} · ${s.mileage} mi · ${s.source}"
                ) {
                    val total = s.laborCost + s.partsCost
                    if (total > 0) Text("$${"%.2f".format(total)}")
                    s.gallons?.let { Text("$it gal") }
                }
            }
        }
    }
}
