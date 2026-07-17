package org.fixitgarage.app.data.repository

import kotlinx.coroutines.flow.Flow
import org.fixitgarage.app.data.dao.ServiceRecordDao
import org.fixitgarage.app.data.dao.VehicleDao
import org.fixitgarage.app.data.entity.ServiceRecordEntity
import org.fixitgarage.app.data.entity.VehicleEntity
import org.fixitgarage.app.util.CsvExporter
import org.fixitgarage.app.util.MpgCalculator

class VehicleRepository(
    private val vehicleDao: VehicleDao,
    private val serviceRecordDao: ServiceRecordDao
) {
    fun observeVehicles(): Flow<List<VehicleEntity>> = vehicleDao.observeActive()

    fun observeVehicle(id: Long): Flow<VehicleEntity?> = vehicleDao.observeById(id)

    fun observeLastService(): Flow<ServiceRecordEntity?> = serviceRecordDao.observeLastService()

    fun observeServices(vehicleId: Long): Flow<List<ServiceRecordEntity>> =
        serviceRecordDao.observeForVehicle(vehicleId)

    suspend fun saveVehicle(vehicle: VehicleEntity): Long = vehicleDao.upsert(vehicle)

    suspend fun saveService(record: ServiceRecordEntity): Long = serviceRecordDao.upsert(record)

    suspend fun deleteVehicle(vehicle: VehicleEntity) = vehicleDao.delete(vehicle)

    /**
     * Automatic MPG from successive fuel fill-ups that include gallons + mileage.
     */
    suspend fun estimateMpg(vehicleId: Long): Double? {
        val records = serviceRecordDao.getAll()
            .filter { it.vehicleId == vehicleId && it.gallons != null && it.gallons > 0 }
            .sortedBy { it.mileage }
        return MpgCalculator.averageMpg(records.map { it.mileage to it.gallons!! })
    }

    suspend fun exportAllCsv(): String {
        val services = serviceRecordDao.getAll()
        return CsvExporter.servicesToCsv(services)
    }
}
