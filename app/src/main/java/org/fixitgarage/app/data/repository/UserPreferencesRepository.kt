package org.fixitgarage.app.data.repository

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import org.fixitgarage.app.domain.model.UserMode

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "user_prefs")

class UserPreferencesRepository(private val context: Context) {

    private val wizardCompletedKey = booleanPreferencesKey("wizard_completed")
    private val userModeKey = stringPreferencesKey("user_mode")
    private val darkModeKey = stringPreferencesKey("dark_mode") // SYSTEM, LIGHT, DARK
    private val selectedVehicleIdKey = stringPreferencesKey("selected_vehicle_id")

    val wizardCompleted: Flow<Boolean> = context.dataStore.data.map { prefs ->
        prefs[wizardCompletedKey] ?: false
    }

    val userMode: Flow<UserMode> = context.dataStore.data.map { prefs ->
        UserMode.fromStorage(prefs[userModeKey])
    }

    val darkMode: Flow<String> = context.dataStore.data.map { prefs ->
        prefs[darkModeKey] ?: "SYSTEM"
    }

    val selectedVehicleId: Flow<Long?> = context.dataStore.data.map { prefs ->
        prefs[selectedVehicleIdKey]?.toLongOrNull()
    }

    suspend fun completeWizard(mode: UserMode) {
        context.dataStore.edit { prefs ->
            prefs[wizardCompletedKey] = true
            prefs[userModeKey] = mode.name
        }
    }

    suspend fun setDarkMode(mode: String) {
        context.dataStore.edit { prefs ->
            prefs[darkModeKey] = mode
        }
    }

    suspend fun setSelectedVehicleId(id: Long?) {
        context.dataStore.edit { prefs ->
            if (id == null) prefs.remove(selectedVehicleIdKey)
            else prefs[selectedVehicleIdKey] = id.toString()
        }
    }
}
