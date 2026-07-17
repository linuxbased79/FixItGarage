package org.fixitgarage.app.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import org.fixitgarage.app.data.dao.ComponentTrackerDao
import org.fixitgarage.app.data.dao.IssuePhotoDao
import org.fixitgarage.app.data.dao.NoteDao
import org.fixitgarage.app.data.dao.PartLogDao
import org.fixitgarage.app.data.dao.ReminderDao
import org.fixitgarage.app.data.dao.ServiceRecordDao
import org.fixitgarage.app.data.dao.TireDao
import org.fixitgarage.app.data.dao.VehicleDao
import org.fixitgarage.app.data.entity.ComponentTrackerEntity
import org.fixitgarage.app.data.entity.IssuePhotoEntity
import org.fixitgarage.app.data.entity.NoteEntity
import org.fixitgarage.app.data.entity.PartLogEntity
import org.fixitgarage.app.data.entity.ReminderEntity
import org.fixitgarage.app.data.entity.ServiceRecordEntity
import org.fixitgarage.app.data.entity.TirePositionEntity
import org.fixitgarage.app.data.entity.TireRotationEntity
import org.fixitgarage.app.data.entity.TireSetEntity
import org.fixitgarage.app.data.entity.VehicleEntity

@Database(
    entities = [
        VehicleEntity::class,
        ServiceRecordEntity::class,
        TireSetEntity::class,
        TirePositionEntity::class,
        TireRotationEntity::class,
        PartLogEntity::class,
        ComponentTrackerEntity::class,
        ReminderEntity::class,
        IssuePhotoEntity::class,
        NoteEntity::class
    ],
    version = 1,
    exportSchema = false
)
abstract class AppDatabase : RoomDatabase() {
    abstract fun vehicleDao(): VehicleDao
    abstract fun serviceRecordDao(): ServiceRecordDao
    abstract fun tireDao(): TireDao
    abstract fun partLogDao(): PartLogDao
    abstract fun componentTrackerDao(): ComponentTrackerDao
    abstract fun reminderDao(): ReminderDao
    abstract fun issuePhotoDao(): IssuePhotoDao
    abstract fun noteDao(): NoteDao

    companion object {
        @Volatile
        private var instance: AppDatabase? = null

        fun get(context: Context): AppDatabase =
            instance ?: synchronized(this) {
                instance ?: Room.databaseBuilder(
                    context.applicationContext,
                    AppDatabase::class.java,
                    "fixitgarage.db"
                ).fallbackToDestructiveMigration()
                    .build()
                    .also { instance = it }
            }
    }
}
