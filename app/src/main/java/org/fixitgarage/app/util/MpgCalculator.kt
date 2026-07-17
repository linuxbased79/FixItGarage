package org.fixitgarage.app.util

/**
 * MPG from consecutive fill-ups: (miles driven) / gallons of the later fill.
 * Assumes full-tank fill-ups.
 */
object MpgCalculator {
    fun averageMpg(fills: List<Pair<Int, Double>>): Double? {
        if (fills.size < 2) return null
        val segments = mutableListOf<Double>()
        for (i in 1 until fills.size) {
            val miles = fills[i].first - fills[i - 1].first
            val gallons = fills[i].second
            if (miles > 0 && gallons > 0) {
                segments += miles / gallons
            }
        }
        if (segments.isEmpty()) return null
        return segments.average()
    }
}
