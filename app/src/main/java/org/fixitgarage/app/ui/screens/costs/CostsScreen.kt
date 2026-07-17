package org.fixitgarage.app.ui.screens.costs

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.fixitgarage.app.data.entity.ServiceRecordEntity
import org.fixitgarage.app.ui.components.SectionCard
import java.util.Calendar

@Composable
fun CostsScreen(
    services: List<ServiceRecordEntity>,
    modifier: Modifier = Modifier
) {
    val now = Calendar.getInstance()
    val thisMonth = services.filter {
        val c = Calendar.getInstance().apply { timeInMillis = it.dateEpochMs }
        c.get(Calendar.YEAR) == now.get(Calendar.YEAR) &&
            c.get(Calendar.MONTH) == now.get(Calendar.MONTH)
    }
    val thisYear = services.filter {
        val c = Calendar.getInstance().apply { timeInMillis = it.dateEpochMs }
        c.get(Calendar.YEAR) == now.get(Calendar.YEAR)
    }

    fun total(list: List<ServiceRecordEntity>): Double =
        list.sumOf { it.laborCost + it.partsCost + (it.fuelCost ?: 0.0) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Operational costs", style = MaterialTheme.typography.headlineMedium)
        Text(
            "Monthly and yearly totals from maintenance, fuel, and parts.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        SectionCard(title = "This month") {
            Text(
                "$${"%.2f".format(total(thisMonth))}",
                style = MaterialTheme.typography.headlineSmall
            )
            Text("${thisMonth.size} records")
        }
        SectionCard(title = "This year") {
            Text(
                "$${"%.2f".format(total(thisYear))}",
                style = MaterialTheme.typography.headlineSmall
            )
            Text("${thisYear.size} records")
        }
        SectionCard(title = "All time") {
            Text(
                "$${"%.2f".format(total(services))}",
                style = MaterialTheme.typography.headlineSmall
            )
        }
    }
}
