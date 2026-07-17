package org.fixitgarage.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.DirectionsCar
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.AttachMoney
import androidx.compose.material.icons.automirrored.filled.ShowChart
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.launch
import org.fixitgarage.app.data.entity.ServiceRecordEntity
import org.fixitgarage.app.domain.model.UserMode
import org.fixitgarage.app.ui.navigation.AppDestination
import org.fixitgarage.app.ui.screens.BatteryScreen
import org.fixitgarage.app.ui.screens.BrakesScreen
import org.fixitgarage.app.ui.screens.ExportScreen
import org.fixitgarage.app.ui.screens.NotesScreen
import org.fixitgarage.app.ui.screens.PhotosScreen
import org.fixitgarage.app.ui.screens.RemindersScreen
import org.fixitgarage.app.ui.screens.WipersScreen
import org.fixitgarage.app.ui.screens.costs.CostsScreen
import org.fixitgarage.app.ui.screens.home.HomeScreen
import org.fixitgarage.app.ui.screens.maintenance.MaintenanceScreen
import org.fixitgarage.app.ui.screens.ocr.OcrScreen
import org.fixitgarage.app.ui.screens.parts.PartsScreen
import org.fixitgarage.app.ui.screens.settings.SettingsScreen
import org.fixitgarage.app.ui.screens.tires.TiresScreen
import org.fixitgarage.app.ui.screens.vehicles.VehiclesScreen
import org.fixitgarage.app.ui.screens.wizard.SetupWizardScreen
import org.fixitgarage.app.ui.theme.FixItGarageTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val app = application as FixItGarageApp
        setContent {
            FixItGarageRoot(app)
        }
    }
}

@Composable
private fun FixItGarageRoot(app: FixItGarageApp) {
    val darkPref by app.preferences.darkMode.collectAsState(initial = "SYSTEM")
    val dark = when (darkPref) {
        "DARK" -> true
        "LIGHT" -> false
        else -> isSystemInDarkTheme()
    }
    FixItGarageTheme(darkTheme = dark) {
        FixItGarageAppContent(app)
    }
}

@Composable
private fun FixItGarageAppContent(app: FixItGarageApp) {
    val scope = rememberCoroutineScope()
    val wizardDone by app.preferences.wizardCompleted.collectAsState(initial = false)
    val userMode by app.preferences.userMode.collectAsState(initial = UserMode.BOTH)
    val darkMode by app.preferences.darkMode.collectAsState(initial = "SYSTEM")
    val vehicles by app.vehicles.observeVehicles().collectAsState(initial = emptyList())
    val lastService by app.vehicles.observeLastService().collectAsState(initial = null)

    val primaryVehicleId = vehicles.firstOrNull()?.id
    val services by remember(primaryVehicleId) {
        if (primaryVehicleId != null) app.vehicles.observeServices(primaryVehicleId)
        else flowOf(emptyList())
    }.collectAsState(initial = emptyList())

    var csvPreview by remember { mutableStateOf("") }

    if (!wizardDone) {
        SetupWizardScreen(
            onComplete = { mode ->
                scope.launch { app.preferences.completeWizard(mode) }
            }
        )
        return
    }

    val navController = rememberNavController()
    val backStack by navController.currentBackStackEntryAsState()
    val currentRoute = backStack?.destination?.route

    val bottomItems: List<Triple<AppDestination, ImageVector, String>> = listOf(
        Triple(AppDestination.Home, Icons.Default.Home, "Home"),
        Triple(AppDestination.Vehicles, Icons.Default.DirectionsCar, "Vehicles"),
        Triple(AppDestination.Maintenance, Icons.Default.Build, "Service"),
        Triple(AppDestination.Tires, Icons.AutoMirrored.Filled.ShowChart, "Tires"),
        Triple(AppDestination.Costs, Icons.Default.AttachMoney, "Costs"),
        Triple(AppDestination.Settings, Icons.Default.Settings, "Settings")
    )

    Scaffold(
        bottomBar = {
            if (currentRoute in bottomItems.map { it.first.route }) {
                NavigationBar {
                    bottomItems.forEach { (dest, icon, label) ->
                        NavigationBarItem(
                            selected = currentRoute == dest.route,
                            onClick = {
                                navController.navigate(dest.route) {
                                    popUpTo(navController.graph.findStartDestination().id) {
                                        saveState = true
                                    }
                                    launchSingleTop = true
                                    restoreState = true
                                }
                            },
                            icon = { Icon(icon, contentDescription = label) },
                            label = { Text(label) }
                        )
                    }
                }
            }
        }
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = AppDestination.Home.route,
            modifier = Modifier.padding(padding)
        ) {
            composable(AppDestination.Home.route) {
                HomeScreen(
                    vehicles = vehicles,
                    lastService = lastService,
                    userMode = userMode,
                    onAddVehicle = { navController.navigate(AppDestination.Vehicles.route) },
                    onScanReceipt = { navController.navigate(AppDestination.Ocr.route) },
                    onOpenTires = { navController.navigate(AppDestination.Tires.route) }
                )
            }
            composable(AppDestination.Vehicles.route) {
                VehiclesScreen(
                    vehicles = vehicles,
                    onSave = { v -> scope.launch { app.vehicles.saveVehicle(v) } }
                )
            }
            composable(AppDestination.Maintenance.route) {
                MaintenanceScreen(
                    vehicles = vehicles,
                    services = services,
                    userMode = userMode,
                    onSave = { s -> scope.launch { app.vehicles.saveService(s) } },
                    onScanReceipt = { navController.navigate(AppDestination.Ocr.route) }
                )
            }
            composable(AppDestination.Tires.route) { TiresScreen() }
            composable(AppDestination.Parts.route) { PartsScreen() }
            composable(AppDestination.Costs.route) { CostsScreen(services = services) }
            composable(AppDestination.Settings.route) {
                SettingsScreen(
                    userMode = userMode,
                    darkMode = darkMode,
                    onDarkModeChange = { mode ->
                        scope.launch { app.preferences.setDarkMode(mode) }
                    },
                    onUserModeChange = { mode ->
                        scope.launch { app.preferences.completeWizard(mode) }
                    },
                    onExport = {
                        scope.launch {
                            csvPreview = app.vehicles.exportAllCsv()
                            navController.navigate(AppDestination.Export.route)
                        }
                    },
                    onOpenReminders = { navController.navigate(AppDestination.Reminders.route) },
                    onOpenPhotos = { navController.navigate(AppDestination.Photos.route) },
                    onOpenNotes = { navController.navigate(AppDestination.Notes.route) },
                    onOpenBrakes = { navController.navigate(AppDestination.Brakes.route) },
                    onOpenBattery = { navController.navigate(AppDestination.Battery.route) },
                    onOpenWipers = { navController.navigate(AppDestination.Wipers.route) }
                )
            }
            composable(AppDestination.Ocr.route) {
                OcrScreen(
                    onSaveParsed = { _, mileage, gallons, cost, _, labor ->
                        val vid = vehicles.firstOrNull()?.id ?: return@OcrScreen
                        scope.launch {
                            app.vehicles.saveService(
                                ServiceRecordEntity(
                                    vehicleId = vid,
                                    dateEpochMs = System.currentTimeMillis(),
                                    mileage = mileage.toIntOrNull() ?: 0,
                                    title = "Receipt import",
                                    source = "SHOP",
                                    partsCost = cost.toDoubleOrNull() ?: 0.0,
                                    laborCost = labor.toDoubleOrNull() ?: 0.0,
                                    gallons = gallons.toDoubleOrNull()
                                )
                            )
                            navController.popBackStack()
                        }
                    }
                )
            }
            composable(AppDestination.Brakes.route) { BrakesScreen() }
            composable(AppDestination.Battery.route) { BatteryScreen() }
            composable(AppDestination.Wipers.route) { WipersScreen() }
            composable(AppDestination.Photos.route) { PhotosScreen() }
            composable(AppDestination.Notes.route) { NotesScreen() }
            composable(AppDestination.Reminders.route) { RemindersScreen() }
            composable(AppDestination.Export.route) { ExportScreen(csvPreview = csvPreview) }
        }
    }
}
