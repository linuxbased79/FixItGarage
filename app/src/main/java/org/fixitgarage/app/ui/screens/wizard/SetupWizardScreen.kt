package org.fixitgarage.app.ui.screens.wizard

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.domain.model.UserMode

@Composable
fun SetupWizardScreen(
    onComplete: (UserMode) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            "Welcome to FixItGarage",
            style = MaterialTheme.typography.headlineMedium
        )
        Text(
            "How do you mostly maintain your vehicles? We'll highlight the tools that fit you. " +
                "You can change this later in Settings.",
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        ModeButton(
            title = "Mostly DIY",
            body = "Oil changes, filters, tread checks, and home-garage logging first.",
            onClick = { onComplete(UserMode.DIY) }
        )
        ModeButton(
            title = "Mostly shop",
            body = "Receipt capture, shop history, labor/parts costs front and center.",
            onClick = { onComplete(UserMode.SHOP) }
        )
        OutlinedButton(
            onClick = { onComplete(UserMode.BOTH) },
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(vertical = 4.dp)) {
                Text("Both DIY and shop")
                Text(
                    "Full feature set — unlimited vehicles, all trackers.",
                    style = MaterialTheme.typography.bodySmall
                )
            }
        }
    }
}

@Composable
private fun ModeButton(title: String, body: String, onClick: () -> Unit) {
    Button(onClick = onClick, modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(vertical = 4.dp)) {
            Text(title)
            Text(body, style = MaterialTheme.typography.bodySmall)
        }
    }
}
