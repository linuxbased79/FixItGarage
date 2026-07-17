package org.fixitgarage.app.ui.screens

import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.ui.components.PlaceholderFeature
import org.fixitgarage.app.util.ReminderScheduler

@Composable
fun BrakesScreen(modifier: Modifier = Modifier) {
    PlaceholderFeature(
        title = "Brake tracker",
        description = "Pads, fluid, and reminders by mileage/date. Room component type BRAKE_*.",
        modifier = modifier.padding(16.dp)
    )
}

@Composable
fun BatteryScreen(modifier: Modifier = Modifier) {
    PlaceholderFeature(
        title = "Battery age tracker",
        description = "Install date and replacement reminders.",
        modifier = modifier.padding(16.dp)
    )
}

@Composable
fun WipersScreen(modifier: Modifier = Modifier) {
    PlaceholderFeature(
        title = "Wiper blade tracker",
        description = "Front/rear install dates and seasonal reminders.",
        modifier = modifier.padding(16.dp)
    )
}

@Composable
fun PhotosScreen(modifier: Modifier = Modifier) {
    PlaceholderFeature(
        title = "Issue photo log",
        description = "CameraX capture stored privately on device, linked to a vehicle.",
        modifier = modifier.padding(16.dp)
    )
}

@Composable
fun NotesScreen(modifier: Modifier = Modifier) {
    PlaceholderFeature(
        title = "Notes",
        description = "Free-form notes per vehicle (NoteEntity in Room).",
        modifier = modifier.padding(16.dp)
    )
}

@Composable
fun RemindersScreen(modifier: Modifier = Modifier) {
    PlaceholderFeature(
        title = "Smart reminders",
        description = "Date and mileage based. Oil level checks every " +
            "${ReminderScheduler.OIL_LEVEL_INTERVAL_MONTHS} months by default.",
        modifier = modifier.padding(16.dp)
    )
}

@Composable
fun ExportScreen(
    csvPreview: String,
    modifier: Modifier = Modifier
) {
    PlaceholderFeature(
        title = "Export CSV",
        description = if (csvPreview.isBlank()) {
            "No service data yet. Export uses CsvExporter over maintenance history."
        } else {
            csvPreview.take(500)
        },
        modifier = modifier.padding(16.dp)
    )
}
