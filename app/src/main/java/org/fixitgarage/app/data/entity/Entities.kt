package org.fixitgarage.app.data.entity

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(tableName = "vehicles")
data class VehicleEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val name: String,
    val make: String = "",
    val model: String = "",
    val year: Int? = null,
    val vin: String = "",
    val licensePlate: String = "",
    val currentMileage: Int = 0,
    val notes: String = "",
    val createdAtEpochMs: Long = System.currentTimeMillis(),
    val isArchived: Boolean = false
)

@Entity(
    tableName = "service_records",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId"), Index("dateEpochMs")]
)
data class ServiceRecordEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    val dateEpochMs: Long,
    val mileage: Int,
    val title: String,
    val description: String = "",
    /** SHOP or DIY */
    val source: String = "DIY",
    val laborCost: Double = 0.0,
    val partsCost: Double = 0.0,
    val gallons: Double? = null,
    val fuelCost: Double? = null,
    val shopName: String = "",
    val receiptImagePath: String? = null,
    val createdAtEpochMs: Long = System.currentTimeMillis()
)

@Entity(
    tableName = "tire_sets",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId")]
)
data class TireSetEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    val brand: String = "",
    val model: String = "",
    val size: String = "",
    val purchaseDateEpochMs: Long? = null,
    val purchaseMileage: Int? = null,
    val cost: Double = 0.0,
    val receiptImagePath: String? = null,
    val notes: String = ""
)

/**
 * Position codes: FL, FR, RL, RR, SP (spare).
 * Rotation log snapshots tire IDs / positions for graphical before/after.
 */
@Entity(
    tableName = "tire_positions",
    foreignKeys = [
        ForeignKey(
            entity = TireSetEntity::class,
            parentColumns = ["id"],
            childColumns = ["tireSetId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("tireSetId")]
)
data class TirePositionEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val tireSetId: Long,
    val position: String,
    val treadDepthMm: Double? = null,
    val mileageOnTire: Int = 0
)

@Entity(
    tableName = "tire_rotations",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId")]
)
data class TireRotationEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    val dateEpochMs: Long,
    val mileage: Int,
    /** e.g. FORWARD_CROSS, REARWARD_CROSS, X_PATTERN, SIDE_TO_SIDE */
    val pattern: String,
    /** JSON map of position -> position before rotation */
    val beforeLayoutJson: String,
    /** JSON map of position -> position after rotation */
    val afterLayoutJson: String,
    val notes: String = ""
)

@Entity(
    tableName = "parts_log",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId"), Index("partType")]
)
data class PartLogEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    /** ENGINE_AIR_FILTER, CABIN_FILTER, OIL_FILTER, OIL_TYPE, OTHER */
    val partType: String,
    val brand: String = "",
    val partNumber: String = "",
    val oilViscosity: String = "",
    val installedDateEpochMs: Long? = null,
    val installedMileage: Int? = null,
    val notes: String = ""
)

@Entity(
    tableName = "component_trackers",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId"), Index("componentType")]
)
data class ComponentTrackerEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    /** WIPER_FRONT, WIPER_REAR, BATTERY, BRAKE_PADS_FRONT, BRAKE_PADS_REAR, BRAKE_FLUID, OIL_LEVEL */
    val componentType: String,
    val installedDateEpochMs: Long? = null,
    val installedMileage: Int? = null,
    val nextDueDateEpochMs: Long? = null,
    val nextDueMileage: Int? = null,
    val notes: String = ""
)

@Entity(
    tableName = "reminders",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId"), Index("dueDateEpochMs")]
)
data class ReminderEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    val title: String,
    val dueDateEpochMs: Long? = null,
    val dueMileage: Int? = null,
    val intervalMonths: Int? = null,
    val intervalMiles: Int? = null,
    val isCompleted: Boolean = false,
    val createdAtEpochMs: Long = System.currentTimeMillis()
)

@Entity(
    tableName = "issue_photos",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId")]
)
data class IssuePhotoEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    val filePath: String,
    val caption: String = "",
    val createdAtEpochMs: Long = System.currentTimeMillis()
)

@Entity(
    tableName = "notes",
    foreignKeys = [
        ForeignKey(
            entity = VehicleEntity::class,
            parentColumns = ["id"],
            childColumns = ["vehicleId"],
            onDelete = ForeignKey.CASCADE
        )
    ],
    indices = [Index("vehicleId")]
)
data class NoteEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val vehicleId: Long,
    val title: String,
    val body: String = "",
    val updatedAtEpochMs: Long = System.currentTimeMillis()
)
