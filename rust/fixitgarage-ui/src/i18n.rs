//! Lightweight language packs for FixItGarage.
//!
//! **Default behavior:** follow the OS language when preference is `SYSTEM`.
//! **Override:** user picks a language in Settings (saved on device).
//!
//! Strings are keyed maps (no gettext) so this works the same on GrapheneOS,
//! desktop, and F-Droid builds without system locale catalogs.

use std::collections::HashMap;
use std::sync::OnceLock;

/// User preference stored in state.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguagePref {
    /// Use device / OS language (recommended default).
    System,
    En,
    Es,
    Fr,
    De,
}

impl LanguagePref {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "EN" | "ENGLISH" | "EN_US" | "EN_GB" => Self::En,
            "ES" | "SPANISH" | "ES_ES" | "ES_MX" => Self::Es,
            "FR" | "FRENCH" | "FR_FR" => Self::Fr,
            "DE" | "GERMAN" | "DE_DE" => Self::De,
            _ => Self::System,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "SYSTEM",
            Self::En => "EN",
            Self::Es => "ES",
            Self::Fr => "FR",
            Self::De => "DE",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System default",
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::De => "Deutsch",
        }
    }
}

/// Resolved language used for lookups (never SYSTEM).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Es,
    Fr,
    De,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
        }
    }

    pub fn from_locale_tag(tag: &str) -> Self {
        let t = tag.trim().to_ascii_lowercase();
        let primary = t.split(['_', '-', '.']).next().unwrap_or("en");
        match primary {
            "es" => Self::Es,
            "fr" => Self::Fr,
            "de" => Self::De,
            _ => Self::En,
        }
    }
}

/// Resolve preference + OS locale into a concrete language pack.
pub fn resolve_lang(pref: LanguagePref, system_locale: &str) -> Lang {
    match pref {
        LanguagePref::System => Lang::from_locale_tag(system_locale),
        LanguagePref::En => Lang::En,
        LanguagePref::Es => Lang::Es,
        LanguagePref::Fr => Lang::Fr,
        LanguagePref::De => Lang::De,
    }
}

/// Translate a key. Falls back to English, then the key itself.
pub fn t(lang: Lang, key: &str) -> String {
    if let Some(s) = pack(lang).get(key) {
        return (*s).to_string();
    }
    if lang != Lang::En {
        if let Some(s) = pack(Lang::En).get(key) {
            return (*s).to_string();
        }
    }
    key.to_string()
}

fn pack(lang: Lang) -> &'static HashMap<&'static str, &'static str> {
    match lang {
        Lang::En => en_map(),
        Lang::Es => es_map(),
        Lang::Fr => fr_map(),
        Lang::De => de_map(),
    }
}

fn en_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            // Nav
            ("nav.home", "Home"),
            ("nav.cars", "Cars"),
            ("nav.service", "Service"),
            ("nav.tires", "Tires"),
            ("nav.costs", "Costs"),
            ("nav.more", "More"),
            ("nav.settings", "Settings"),
            // Common
            ("app.title", "FixItGarage"),
            ("common.save", "Save"),
            ("common.delete", "Delete"),
            ("common.back", "Back"),
            ("common.back_more", "Back to More"),
            ("common.switch", "Switch"),
            // Settings
            ("settings.title", "⚙ Settings"),
            ("settings.intro", "App preferences. Trackers and tools live under More."),
            ("settings.appearance", "Appearance"),
            ("settings.appearance_body", "Dark is the default. Your choice is remembered on this device."),
            ("settings.dark", "Dark"),
            ("settings.light", "Light"),
            ("settings.units", "Units of measure"),
            ("settings.units_body", "Choose imperial or metric. Existing data is converted for display."),
            ("settings.imperial", "Imperial"),
            ("settings.metric", "Metric"),
            ("settings.language", "Language"),
            ("settings.language_body", "System default follows your phone language. Or pick a language pack for FixItGarage only."),
            ("settings.lang_system", "System default"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.feature_focus", "Feature focus"),
            ("settings.feature_body", "Hides DIY-only or shop-only tools on the main tabs and More."),
            ("settings.data", "Data & backup"),
            ("settings.data_body", "Local-first. Create a JSON backup, then send it to your cloud app."),
            ("settings.cloud", "Cloud apps (recommended)"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "Support"),
            ("settings.about", "About"),
            ("settings.donate", "Donate"),
            ("settings.feedback", "Send feedback (GitHub Issues)"),
            // More
            ("more.title", "More"),
            ("more.intro", "Trackers and tools for the selected vehicle. App preferences are under Settings (gear)."),
            ("more.trackers", "Maintenance trackers"),
            ("more.logs", "Logs & reminders"),
            ("more.quick", "Quick links"),
            ("more.open_settings", "⚙ Open Settings"),
            // Home
            ("home.last_service", "Last service"),
            ("home.vehicles", "Vehicles"),
            ("home.quick_actions", "Quick actions"),
            ("home.at_a_glance", "At a glance"),
            ("home.upcoming", "Upcoming (90 days / 5k mi)"),
            // Status
            ("status.language_set", "Language saved."),
            ("status.language_system", "Using system language."),
        ])
    })
}

fn es_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            ("nav.home", "Inicio"),
            ("nav.cars", "Autos"),
            ("nav.service", "Servicio"),
            ("nav.tires", "Llantas"),
            ("nav.costs", "Costos"),
            ("nav.more", "Más"),
            ("nav.settings", "Ajustes"),
            ("app.title", "FixItGarage"),
            ("common.save", "Guardar"),
            ("common.delete", "Eliminar"),
            ("common.back", "Atrás"),
            ("common.back_more", "Volver a Más"),
            ("common.switch", "Cambiar"),
            ("settings.title", "⚙ Ajustes"),
            ("settings.intro", "Preferencias de la app. Rastreadores y herramientas están en Más."),
            ("settings.appearance", "Apariencia"),
            ("settings.appearance_body", "Oscuro es el valor predeterminado. Se guarda en este dispositivo."),
            ("settings.dark", "Oscuro"),
            ("settings.light", "Claro"),
            ("settings.units", "Unidades"),
            ("settings.units_body", "Elige imperial o métrico. Los datos se convierten al mostrarlos."),
            ("settings.imperial", "Imperial"),
            ("settings.metric", "Métrico"),
            ("settings.language", "Idioma"),
            ("settings.language_body", "El predeterminado del sistema sigue el idioma del teléfono. O elige un paquete solo para FixItGarage."),
            ("settings.lang_system", "Idioma del sistema"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.feature_focus", "Enfoque"),
            ("settings.feature_body", "Oculta herramientas solo DIY o solo taller en las pestañas y Más."),
            ("settings.data", "Datos y copia"),
            ("settings.data_body", "Local primero. Crea una copia JSON y envíala a tu nube."),
            ("settings.cloud", "Apps de nube (recomendado)"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "Soporte"),
            ("settings.about", "Acerca de"),
            ("settings.donate", "Donar"),
            ("settings.feedback", "Enviar comentarios (GitHub Issues)"),
            ("more.title", "Más"),
            ("more.intro", "Rastreadores y herramientas del vehículo. Preferencias en Ajustes (engranaje)."),
            ("more.trackers", "Rastreadores"),
            ("more.logs", "Registros y recordatorios"),
            ("more.quick", "Accesos rápidos"),
            ("more.open_settings", "⚙ Abrir Ajustes"),
            ("home.last_service", "Último servicio"),
            ("home.vehicles", "Vehículos"),
            ("home.quick_actions", "Acciones rápidas"),
            ("home.at_a_glance", "Resumen"),
            ("home.upcoming", "Próximos (90 días / 5k)"),
            ("status.language_set", "Idioma guardado."),
            ("status.language_system", "Usando el idioma del sistema."),
        ])
    })
}

fn fr_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            ("nav.home", "Accueil"),
            ("nav.cars", "Véhicules"),
            ("nav.service", "Service"),
            ("nav.tires", "Pneus"),
            ("nav.costs", "Coûts"),
            ("nav.more", "Plus"),
            ("nav.settings", "Réglages"),
            ("app.title", "FixItGarage"),
            ("common.save", "Enregistrer"),
            ("common.delete", "Supprimer"),
            ("common.back", "Retour"),
            ("common.back_more", "Retour à Plus"),
            ("common.switch", "Changer"),
            ("settings.title", "⚙ Réglages"),
            ("settings.intro", "Préférences de l’app. Outils et suivis dans Plus."),
            ("settings.appearance", "Apparence"),
            ("settings.appearance_body", "Sombre par défaut. Choix mémorisé sur cet appareil."),
            ("settings.dark", "Sombre"),
            ("settings.light", "Clair"),
            ("settings.units", "Unités"),
            ("settings.units_body", "Impérial ou métrique. Conversion à l’affichage."),
            ("settings.imperial", "Impérial"),
            ("settings.metric", "Métrique"),
            ("settings.language", "Langue"),
            ("settings.language_body", "Par défaut, suit la langue du téléphone. Ou choisissez un pack pour FixItGarage."),
            ("settings.lang_system", "Langue système"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.feature_focus", "Focus"),
            ("settings.feature_body", "Masque les outils DIY ou atelier selon le mode."),
            ("settings.data", "Données et sauvegarde"),
            ("settings.data_body", "Local d’abord. Créez une sauvegarde JSON puis envoyez-la au cloud."),
            ("settings.cloud", "Apps cloud (recommandé)"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "Support"),
            ("settings.about", "À propos"),
            ("settings.donate", "Faire un don"),
            ("settings.feedback", "Commentaires (GitHub Issues)"),
            ("more.title", "Plus"),
            ("more.intro", "Suivis et outils du véhicule. Préférences dans Réglages."),
            ("more.trackers", "Suivis d’entretien"),
            ("more.logs", "Journaux et rappels"),
            ("more.quick", "Raccourcis"),
            ("more.open_settings", "⚙ Ouvrir Réglages"),
            ("home.last_service", "Dernier service"),
            ("home.vehicles", "Véhicules"),
            ("home.quick_actions", "Actions rapides"),
            ("home.at_a_glance", "Aperçu"),
            ("home.upcoming", "À venir (90 j / 5k)"),
            ("status.language_set", "Langue enregistrée."),
            ("status.language_system", "Langue système utilisée."),
        ])
    })
}

fn de_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            ("nav.home", "Start"),
            ("nav.cars", "Autos"),
            ("nav.service", "Service"),
            ("nav.tires", "Reifen"),
            ("nav.costs", "Kosten"),
            ("nav.more", "Mehr"),
            ("nav.settings", "Einstellungen"),
            ("app.title", "FixItGarage"),
            ("common.save", "Speichern"),
            ("common.delete", "Löschen"),
            ("common.back", "Zurück"),
            ("common.back_more", "Zurück zu Mehr"),
            ("common.switch", "Wechseln"),
            ("settings.title", "⚙ Einstellungen"),
            ("settings.intro", "App-Einstellungen. Tracker und Tools unter Mehr."),
            ("settings.appearance", "Erscheinungsbild"),
            ("settings.appearance_body", "Dunkel ist Standard. Wird auf diesem Gerät gespeichert."),
            ("settings.dark", "Dunkel"),
            ("settings.light", "Hell"),
            ("settings.units", "Maßeinheiten"),
            ("settings.units_body", "Imperial oder metrisch. Anzeige wird umgerechnet."),
            ("settings.imperial", "Imperial"),
            ("settings.metric", "Metrisch"),
            ("settings.language", "Sprache"),
            ("settings.language_body", "Systemstandard folgt der Telefonsprache. Oder ein Sprachpaket nur für FixItGarage wählen."),
            ("settings.lang_system", "Systemstandard"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.feature_focus", "Fokus"),
            ("settings.feature_body", "Blendet DIY- oder Werkstatt-Tools aus."),
            ("settings.data", "Daten & Backup"),
            ("settings.data_body", "Lokal zuerst. JSON-Backup erstellen und in die Cloud senden."),
            ("settings.cloud", "Cloud-Apps (empfohlen)"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "Support"),
            ("settings.about", "Über"),
            ("settings.donate", "Spenden"),
            ("settings.feedback", "Feedback (GitHub Issues)"),
            ("more.title", "Mehr"),
            ("more.intro", "Tracker und Tools für das Fahrzeug. Einstellungen über das Zahnrad."),
            ("more.trackers", "Wartungs-Tracker"),
            ("more.logs", "Protokolle & Erinnerungen"),
            ("more.quick", "Schnellzugriff"),
            ("more.open_settings", "⚙ Einstellungen"),
            ("home.last_service", "Letzter Service"),
            ("home.vehicles", "Fahrzeuge"),
            ("home.quick_actions", "Schnellaktionen"),
            ("home.at_a_glance", "Überblick"),
            ("home.upcoming", "Demnächst (90 Tage / 5k)"),
            ("status.language_set", "Sprache gespeichert."),
            ("status.language_system", "Systemsprache wird verwendet."),
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_pref_uses_locale() {
        assert_eq!(resolve_lang(LanguagePref::System, "es_MX"), Lang::Es);
        assert_eq!(resolve_lang(LanguagePref::System, "de-DE"), Lang::De);
        assert_eq!(resolve_lang(LanguagePref::System, "en_US"), Lang::En);
    }

    #[test]
    fn override_ignores_os() {
        assert_eq!(resolve_lang(LanguagePref::Fr, "en_US"), Lang::Fr);
    }

    #[test]
    fn spanish_nav() {
        assert_eq!(t(Lang::Es, "nav.home"), "Inicio");
        assert_eq!(t(Lang::Es, "nav.settings"), "Ajustes");
    }

    #[test]
    fn unknown_key_falls_back_to_english_then_key() {
        assert_eq!(t(Lang::Es, "nav.home"), "Inicio");
        // missing key in all packs → key
        assert_eq!(t(Lang::Es, "no.such.key"), "no.such.key");
    }
}
