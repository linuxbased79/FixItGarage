# FixItGarage ProGuard rules
-keep class org.fixitgarage.app.data.entity.** { *; }
-keep class * extends androidx.room.RoomDatabase
-dontwarn org.bouncycastle.**
