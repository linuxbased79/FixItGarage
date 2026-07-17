package org.fixitgarage.app.util

import org.fixitgarage.app.data.entity.ServiceRecordEntity
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

object CsvExporter {
    private val dateFmt = SimpleDateFormat("yyyy-MM-dd", Locale.US)

    fun servicesToCsv(records: List<ServiceRecordEntity>): String {
        val header = listOf(
            "id", "vehicleId", "date", "mileage", "title", "source",
            "laborCost", "partsCost", "gallons", "fuelCost", "shopName"
        ).joinToString(",")
        val rows = records.map { r ->
            listOf(
                r.id,
                r.vehicleId,
                dateFmt.format(Date(r.dateEpochMs)),
                r.mileage,
                escape(r.title),
                r.source,
                r.laborCost,
                r.partsCost,
                r.gallons ?: "",
                r.fuelCost ?: "",
                escape(r.shopName)
            ).joinToString(",")
        }
        return (listOf(header) + rows).joinToString("\n")
    }

    private fun escape(value: String): String {
        val needsQuotes = value.contains(',') || value.contains('"') || value.contains('\n')
        val escaped = value.replace("\"", "\"\"")
        return if (needsQuotes) "\"$escaped\"" else escaped
    }
}
