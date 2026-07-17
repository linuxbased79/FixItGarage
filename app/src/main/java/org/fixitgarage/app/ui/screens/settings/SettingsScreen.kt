package org.fixitgarage.app.ui.screens.settings

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.domain.model.UserMode
import org.fixitgarage.app.ui.components.SectionCard

@Composable
fun SettingsScreen(
    userMode: UserMode,
    darkMode: String,
    onDarkModeChange: (String) -> Unit,
    onUserModeChange: (UserMode) -> Unit,
    onExport: () -> Unit,
    onOpenReminders: () -> Unit,
    onOpenPhotos: () -> Unit,
    onOpenNotes: () -> Unit,
    onOpenBrakes: () -> Unit,
    onOpenBattery: () -> Unit,
    onOpenWipers: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current

    fun openUrl(url: String) {
        context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Settings", style = MaterialTheme.typography.headlineMedium)

        SectionCard(title = "Appearance", subtitle = "Full dark mode support") {
            androidx.compose.foundation.layout.Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                listOf("SYSTEM" to "System", "LIGHT" to "Light", "DARK" to "Dark").forEach { (key, label) ->
                    FilterChip(
                        selected = darkMode == key,
                        onClick = { onDarkModeChange(key) },
                        label = { Text(label) }
                    )
                }
            }
        }

        SectionCard(title = "Feature focus") {
            androidx.compose.foundation.layout.Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                UserMode.entries.forEach { mode ->
                    FilterChip(
                        selected = userMode == mode,
                        onClick = { onUserModeChange(mode) },
                        label = { Text(mode.name) }
                    )
                }
            }
        }

        SectionCard(title = "Trackers & logs") {
            OutlinedButton(onClick = onOpenReminders, modifier = Modifier.fillMaxWidth()) {
                Text("Smart reminders (date + mileage)")
            }
            OutlinedButton(onClick = onOpenBrakes, modifier = Modifier.fillMaxWidth()) {
                Text("Brake tracker")
            }
            OutlinedButton(onClick = onOpenBattery, modifier = Modifier.fillMaxWidth()) {
                Text("Battery age tracker")
            }
            OutlinedButton(onClick = onOpenWipers, modifier = Modifier.fillMaxWidth()) {
                Text("Wiper blade tracker")
            }
            OutlinedButton(onClick = onOpenPhotos, modifier = Modifier.fillMaxWidth()) {
                Text("Issue photo log")
            }
            OutlinedButton(onClick = onOpenNotes, modifier = Modifier.fillMaxWidth()) {
                Text("Notes")
            }
            OutlinedButton(onClick = onExport, modifier = Modifier.fillMaxWidth()) {
                Text("Export data as CSV")
            }
        }

        SectionCard(
            title = "Optional cloud backup",
            subtitle = "Proton Drive, Google Drive, Dropbox, OneDrive, ownCloud, Nextcloud — planned; local-first by default."
        ) {
            Text(
                "No cloud account required. GrapheneOS users can stay fully offline.",
                style = MaterialTheme.typography.bodySmall
            )
        }

        SectionCard(title = "Support the project") {
            Button(
                onClick = { openUrl("https://github.com/linuxbased79/FixItGarage#donate") },
                modifier = Modifier.fillMaxWidth()
            ) { Text("Donate") }
            OutlinedButton(
                onClick = { openUrl("https://github.com/linuxbased79/FixItGarage/issues") },
                modifier = Modifier.fillMaxWidth()
            ) { Text("Send feedback (GitHub Issues)") }
        }

        SectionCard(title = "About") {
            Text("FixItGarage 0.1.0-alpha")
            Text("License: GNU GPL v3.0")
            Text(
                "Compatible with GrapheneOS. Planned for F-Droid and Google Play.",
                style = MaterialTheme.typography.bodySmall
            )
        }
    }
}
