package org.fixitgarage.app.ui.screens.vehicles

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
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
import org.fixitgarage.app.data.entity.VehicleEntity
import org.fixitgarage.app.ui.components.SectionCard

@Composable
fun VehiclesScreen(
    vehicles: List<VehicleEntity>,
    onSave: (VehicleEntity) -> Unit,
    modifier: Modifier = Modifier
) {
    var name by remember { mutableStateOf("") }
    var make by remember { mutableStateOf("") }
    var model by remember { mutableStateOf("") }
    var year by remember { mutableStateOf("") }
    var mileage by remember { mutableStateOf("") }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Vehicles", style = MaterialTheme.typography.headlineMedium)

        SectionCard(title = "Add vehicle") {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                label = { Text("Name / nickname") },
                modifier = Modifier.fillMaxWidth()
            )
            OutlinedTextField(
                value = make,
                onValueChange = { make = it },
                label = { Text("Make") },
                modifier = Modifier.fillMaxWidth()
            )
            OutlinedTextField(
                value = model,
                onValueChange = { model = it },
                label = { Text("Model") },
                modifier = Modifier.fillMaxWidth()
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = year,
                    onValueChange = { year = it.filter { ch -> ch.isDigit() }.take(4) },
                    label = { Text("Year") },
                    modifier = Modifier.weight(1f)
                )
                OutlinedTextField(
                    value = mileage,
                    onValueChange = { mileage = it.filter { ch -> ch.isDigit() } },
                    label = { Text("Mileage") },
                    modifier = Modifier.weight(1f)
                )
            }
            Button(
                onClick = {
                    if (name.isBlank()) return@Button
                    onSave(
                        VehicleEntity(
                            name = name.trim(),
                            make = make.trim(),
                            model = model.trim(),
                            year = year.toIntOrNull(),
                            currentMileage = mileage.toIntOrNull() ?: 0
                        )
                    )
                    name = ""; make = ""; model = ""; year = ""; mileage = ""
                }
            ) { Text("Save vehicle") }
        }

        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(vehicles, key = { it.id }) { v ->
                SectionCard(
                    title = v.name,
                    subtitle = listOfNotNull(
                        v.year?.toString(),
                        v.make.ifBlank { null },
                        v.model.ifBlank { null }
                    ).joinToString(" ").ifBlank { "No make/model" }
                ) {
                    Text("${v.currentMileage} mi")
                }
            }
        }
    }
}
