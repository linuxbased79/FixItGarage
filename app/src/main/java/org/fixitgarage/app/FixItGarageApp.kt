package org.fixitgarage.app

import android.app.Application
import org.fixitgarage.app.data.db.AppDatabase
import org.fixitgarage.app.data.repository.UserPreferencesRepository
import org.fixitgarage.app.data.repository.VehicleRepository

class FixItGarageApp : Application() {
    lateinit var database: AppDatabase
        private set
    lateinit var preferences: UserPreferencesRepository
        private set
    lateinit var vehicles: VehicleRepository
        private set

    override fun onCreate() {
        super.onCreate()
        database = AppDatabase.get(this)
        preferences = UserPreferencesRepository(this)
        vehicles = VehicleRepository(
            vehicleDao = database.vehicleDao(),
            serviceRecordDao = database.serviceRecordDao()
        )
    }
}
