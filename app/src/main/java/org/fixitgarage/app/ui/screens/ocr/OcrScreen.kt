package org.fixitgarage.app.ui.screens.ocr

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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
import org.fixitgarage.app.ui.components.SectionCard

/**
 * Receipt OCR pipeline target fields: date, mileage, gallons, cost, parts, labor.
 *
 * F-Droid / GrapheneOS path: prefer fully free on-device OCR (e.g. Tesseract /
 * ML Kit open alternatives) without requiring Google Play Services.
 * Play Store flavor may optionally use ML Kit Text Recognition.
 */
@Composable
fun OcrScreen(
    onSaveParsed: (
        date: String,
        mileage: String,
        gallons: String,
        cost: String,
        parts: String,
        labor: String
    ) -> Unit,
    modifier: Modifier = Modifier
) {
    var date by remember { mutableStateOf("") }
    var mileage by remember { mutableStateOf("") }
    var gallons by remember { mutableStateOf("") }
    var cost by remember { mutableStateOf("") }
    var parts by remember { mutableStateOf("") }
    var labor by remember { mutableStateOf("") }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Receipt scan (OCR)", style = MaterialTheme.typography.headlineMedium)
        SectionCard(
            title = "On-device OCR",
            subtitle = "Camera capture + free OCR will fill these fields automatically."
        ) {
            Button(onClick = { /* CameraX capture — next iteration */ }, enabled = false) {
                Text("Take photo (coming soon)")
            }
            Text(
                "Until OCR lands, enter values manually from the receipt.",
                style = MaterialTheme.typography.bodySmall
            )
        }
        OutlinedTextField(date, { date = it }, label = { Text("Date") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(mileage, { mileage = it }, label = { Text("Mileage") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(gallons, { gallons = it }, label = { Text("Gallons") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(cost, { cost = it }, label = { Text("Total cost") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(parts, { parts = it }, label = { Text("Parts") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(labor, { labor = it }, label = { Text("Labor") }, modifier = Modifier.fillMaxWidth())
        Button(
            onClick = { onSaveParsed(date, mileage, gallons, cost, parts, labor) }
        ) { Text("Save to maintenance") }
    }
}
