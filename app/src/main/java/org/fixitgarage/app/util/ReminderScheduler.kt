package org.fixitgarage.app.util

/**
 * Helpers for smart reminders (mileage + date).
 * Oil-level checks default to every 3 months per product requirements.
 */
object ReminderScheduler {
    const val OIL_LEVEL_INTERVAL_MONTHS = 3

    fun addMonths(epochMs: Long, months: Int): Long {
        val cal = java.util.Calendar.getInstance()
        cal.timeInMillis = epochMs
        cal.add(java.util.Calendar.MONTH, months)
        return cal.timeInMillis
    }

    fun isDueByDate(dueEpochMs: Long?, now: Long = System.currentTimeMillis()): Boolean =
        dueEpochMs != null && dueEpochMs <= now

    fun isDueByMileage(dueMileage: Int?, currentMileage: Int): Boolean =
        dueMileage != null && currentMileage >= dueMileage
}
