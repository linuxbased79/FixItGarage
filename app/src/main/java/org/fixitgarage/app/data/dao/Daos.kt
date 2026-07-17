package org.fixitgarage.app.data.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import androidx.room.Update
import kotlinx.coroutines.flow.Flow
import org.fixitgarage.app.data.entity.ComponentTrackerEntity
import org.fixitgarage.app.data.entity.IssuePhotoEntity
import org.fixitgarage.app.data.entity.NoteEntity
import org.fixitgarage.app.data.entity.PartLogEntity
import org.fixitgarage.app.data.entity.ReminderEntity
import org.fixitgarage.app.data.entity.ServiceRecordEntity
import org.fixitgarage.app.data.entity.TireRotationEntity
import org.fixitgarage.app.data.entity.TireSetEntity
import org.fixitgarage.app.data.entity.VehicleEntity

@Dao
interface VehicleDao {
    @Query("SELECT * FROM vehicles WHERE isArchived = 0 ORDER BY name ASC")
    fun observeActive(): Flow<List<VehicleEntity>>

    @Query("SELECT * FROM vehicles WHERE id = :id")
    fun observeById(id: Long): Flow<VehicleEntity?>

    @Query("SELECT * FROM vehicles WHERE id = :id")
    suspend fun getById(id: Long): VehicleEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(vehicle: VehicleEntity): Long

    @Update
    suspend fun update(vehicle: VehicleEntity)

    @Delete
    suspend fun delete(vehicle: VehicleEntity)

    @Query("SELECT COUNT(*) FROM vehicles WHERE isArchived = 0")
    suspend fun countActive(): Int
}

@Dao
interface ServiceRecordDao {
    @Query(
        """
        SELECT * FROM service_records
        WHERE vehicleId = :vehicleId
        ORDER BY dateEpochMs DESC, id DESC
        """
    )
    fun observeForVehicle(vehicleId: Long): Flow<List<ServiceRecordEntity>>

    @Query(
        """
        SELECT * FROM service_records
        ORDER BY dateEpochMs DESC, id DESC
        LIMIT 1
        """
    )
    fun observeLastService(): Flow<ServiceRecordEntity?>

    @Query(
        """
        SELECT * FROM service_records
        WHERE vehicleId = :vehicleId
        ORDER BY dateEpochMs DESC, id DESC
        LIMIT 1
        """
    )
    fun observeLastServiceForVehicle(vehicleId: Long): Flow<ServiceRecordEntity?>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(record: ServiceRecordEntity): Long

    @Delete
    suspend fun delete(record: ServiceRecordEntity)

    @Query("SELECT * FROM service_records ORDER BY dateEpochMs ASC")
    suspend fun getAll(): List<ServiceRecordEntity>
}

@Dao
interface TireDao {
    @Query("SELECT * FROM tire_sets WHERE vehicleId = :vehicleId")
    fun observeSets(vehicleId: Long): Flow<List<TireSetEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertSet(set: TireSetEntity): Long

    @Query("SELECT * FROM tire_rotations WHERE vehicleId = :vehicleId ORDER BY dateEpochMs DESC")
    fun observeRotations(vehicleId: Long): Flow<List<TireRotationEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertRotation(rotation: TireRotationEntity): Long
}

@Dao
interface PartLogDao {
    @Query("SELECT * FROM parts_log WHERE vehicleId = :vehicleId ORDER BY partType ASC")
    fun observeForVehicle(vehicleId: Long): Flow<List<PartLogEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(part: PartLogEntity): Long

    @Delete
    suspend fun delete(part: PartLogEntity)
}

@Dao
interface ComponentTrackerDao {
    @Query("SELECT * FROM component_trackers WHERE vehicleId = :vehicleId")
    fun observeForVehicle(vehicleId: Long): Flow<List<ComponentTrackerEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(item: ComponentTrackerEntity): Long
}

@Dao
interface ReminderDao {
    @Query(
        """
        SELECT * FROM reminders
        WHERE isCompleted = 0
        ORDER BY dueDateEpochMs IS NULL, dueDateEpochMs ASC
        """
    )
    fun observeOpen(): Flow<List<ReminderEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(reminder: ReminderEntity): Long

    @Update
    suspend fun update(reminder: ReminderEntity)
}

@Dao
interface IssuePhotoDao {
    @Query("SELECT * FROM issue_photos WHERE vehicleId = :vehicleId ORDER BY createdAtEpochMs DESC")
    fun observeForVehicle(vehicleId: Long): Flow<List<IssuePhotoEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(photo: IssuePhotoEntity): Long
}

@Dao
interface NoteDao {
    @Query("SELECT * FROM notes WHERE vehicleId = :vehicleId ORDER BY updatedAtEpochMs DESC")
    fun observeForVehicle(vehicleId: Long): Flow<List<NoteEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(note: NoteEntity): Long

    @Delete
    suspend fun delete(note: NoteEntity)
}
