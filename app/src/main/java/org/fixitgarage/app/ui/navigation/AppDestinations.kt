package org.fixitgarage.app.ui.navigation

sealed class AppDestination(val route: String, val label: String) {
    data object Home : AppDestination("home", "Home")
    data object Vehicles : AppDestination("vehicles", "Vehicles")
    data object Maintenance : AppDestination("maintenance", "Maintenance")
    data object Tires : AppDestination("tires", "Tires")
    data object Parts : AppDestination("parts", "Parts")
    data object Costs : AppDestination("costs", "Costs")
    data object Settings : AppDestination("settings", "Settings")
    data object Wizard : AppDestination("wizard", "Setup")
    data object Ocr : AppDestination("ocr", "Scan receipt")
    data object Brakes : AppDestination("brakes", "Brakes")
    data object Battery : AppDestination("battery", "Battery")
    data object Wipers : AppDestination("wipers", "Wipers")
    data object Photos : AppDestination("photos", "Photos")
    data object Notes : AppDestination("notes", "Notes")
    data object Reminders : AppDestination("reminders", "Reminders")
    data object Export : AppDestination("export", "Export")

    companion object {
        val bottomNav = listOf(Home, Vehicles, Maintenance, Tires, Costs, Settings)
    }
}
