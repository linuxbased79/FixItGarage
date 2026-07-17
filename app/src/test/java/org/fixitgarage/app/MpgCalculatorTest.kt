package org.fixitgarage.app

import org.fixitgarage.app.util.MpgCalculator
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MpgCalculatorTest {
    @Test
    fun averageMpg_requiresTwoFills() {
        assertNull(MpgCalculator.averageMpg(listOf(1000 to 10.0)))
    }

    @Test
    fun averageMpg_computesSegmentAverage() {
        // 300 miles / 10 gal = 30; 280 miles / 10 gal = 28 → avg 29
        val mpg = MpgCalculator.averageMpg(
            listOf(
                10000 to 10.0,
                10300 to 10.0,
                10580 to 10.0
            )
        )
        assertEquals(29.0, mpg!!, 0.01)
    }
}
