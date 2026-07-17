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
    /// Japanese 日本語
    Ja,
    /// Korean 한국어
    Ko,
    /// Chinese Simplified 简体中文
    Zh,
}

impl LanguagePref {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "EN" | "ENGLISH" | "EN_US" | "EN_GB" => Self::En,
            "ES" | "SPANISH" | "ES_ES" | "ES_MX" => Self::Es,
            "FR" | "FRENCH" | "FR_FR" => Self::Fr,
            "DE" | "GERMAN" | "DE_DE" => Self::De,
            "JA" | "JP" | "JAPANESE" | "JA_JP" => Self::Ja,
            "KO" | "KR" | "KOREAN" | "KO_KR" => Self::Ko,
            "ZH" | "ZH_CN" | "ZH_HANS" | "CHINESE" | "CN" => Self::Zh,
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
            Self::Ja => "JA",
            Self::Ko => "KO",
            Self::Zh => "ZH",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System default",
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::De => "Deutsch",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::Zh => "简体中文",
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
    Ja,
    Ko,
    Zh,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Zh => "zh",
        }
    }

    pub fn from_locale_tag(tag: &str) -> Self {
        let t = tag.trim().to_ascii_lowercase();
        // BCP-47 style: ja-JP, zh-Hans-CN, ko_KR
        let primary = t.split(['_', '-', '.']).next().unwrap_or("en");
        match primary {
            "es" => Self::Es,
            "fr" => Self::Fr,
            "de" => Self::De,
            "ja" => Self::Ja,
            "ko" => Self::Ko,
            "zh" => Self::Zh, // simplified pack for zh / zh-Hans / zh-CN
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
        LanguagePref::Ja => Lang::Ja,
        LanguagePref::Ko => Lang::Ko,
        LanguagePref::Zh => Lang::Zh,
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
        Lang::Ja => ja_map(),
        Lang::Ko => ko_map(),
        Lang::Zh => zh_map(),
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
            ("settings.lang_ja", "日本語"),
            ("settings.lang_ko", "한국어"),
            ("settings.lang_zh", "简体中文"),
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

fn ja_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            ("nav.home", "ホーム"),
            ("nav.cars", "車両"),
            ("nav.service", "整備"),
            ("nav.tires", "タイヤ"),
            ("nav.costs", "費用"),
            ("nav.more", "その他"),
            ("nav.settings", "設定"),
            ("app.title", "FixItGarage"),
            ("common.save", "保存"),
            ("common.delete", "削除"),
            ("common.back", "戻る"),
            ("common.back_more", "その他へ戻る"),
            ("common.switch", "切替"),
            ("settings.title", "⚙ 設定"),
            ("settings.intro", "アプリの設定。追跡とツールは「その他」にあります。"),
            ("settings.appearance", "外観"),
            ("settings.appearance_body", "ダークが既定です。この端末に保存されます。"),
            ("settings.dark", "ダーク"),
            ("settings.light", "ライト"),
            ("settings.units", "単位"),
            ("settings.units_body", "ヤード・ポンド法またはメートル法を選択。表示時に換算します。"),
            ("settings.imperial", "ヤード・ポンド"),
            ("settings.metric", "メートル法"),
            ("settings.language", "言語"),
            ("settings.language_body", "システム既定は端末の言語に従います。または FixItGarage だけの言語パックを選べます。"),
            ("settings.lang_system", "システム既定"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.lang_ja", "日本語"),
            ("settings.lang_ko", "한국어"),
            ("settings.lang_zh", "简体中文"),
            ("settings.feature_focus", "機能フォーカス"),
            ("settings.feature_body", "DIY 専用またはショップ専用のツールを非表示にします。"),
            ("settings.data", "データとバックアップ"),
            ("settings.data_body", "端末優先。JSON バックアップを作成してクラウドへ送れます。"),
            ("settings.cloud", "クラウドアプリ（推奨）"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "サポート"),
            ("settings.about", "情報"),
            ("settings.donate", "寄付"),
            ("settings.feedback", "フィードバック（GitHub Issues）"),
            ("more.title", "その他"),
            ("more.intro", "選択中の車両の追跡とツール。設定は歯車から。"),
            ("more.trackers", "メンテナンストラッカー"),
            ("more.logs", "記録とリマインダー"),
            ("more.quick", "クイックリンク"),
            ("more.open_settings", "⚙ 設定を開く"),
            ("home.last_service", "前回の整備"),
            ("home.vehicles", "車両"),
            ("home.quick_actions", "クイック操作"),
            ("home.at_a_glance", "概要"),
            ("home.upcoming", "予定（90日 / 5k）"),
            ("status.language_set", "言語を保存しました。"),
            ("status.language_system", "システム言語を使用します。"),
        ])
    })
}

fn ko_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            ("nav.home", "홈"),
            ("nav.cars", "차량"),
            ("nav.service", "정비"),
            ("nav.tires", "타이어"),
            ("nav.costs", "비용"),
            ("nav.more", "더보기"),
            ("nav.settings", "설정"),
            ("app.title", "FixItGarage"),
            ("common.save", "저장"),
            ("common.delete", "삭제"),
            ("common.back", "뒤로"),
            ("common.back_more", "더보기로 돌아가기"),
            ("common.switch", "전환"),
            ("settings.title", "⚙ 설정"),
            ("settings.intro", "앱 환경설정. 추적 도구는 더보기에 있습니다."),
            ("settings.appearance", "화면"),
            ("settings.appearance_body", "어두운 모드가 기본입니다. 이 기기에 저장됩니다."),
            ("settings.dark", "어두움"),
            ("settings.light", "밝음"),
            ("settings.units", "단위"),
            ("settings.units_body", "야드파운드 또는 미터법을 선택하세요. 표시 시 변환됩니다."),
            ("settings.imperial", "야드파운드"),
            ("settings.metric", "미터법"),
            ("settings.language", "언어"),
            ("settings.language_body", "시스템 기본은 휴대폰 언어를 따릅니다. 또는 FixItGarage 전용 언어 팩을 고를 수 있습니다."),
            ("settings.lang_system", "시스템 기본"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.lang_ja", "日本語"),
            ("settings.lang_ko", "한국어"),
            ("settings.lang_zh", "简体中文"),
            ("settings.feature_focus", "기능 초점"),
            ("settings.feature_body", "DIY 전용 또는 정비소 전용 도구를 숨깁니다."),
            ("settings.data", "데이터 및 백업"),
            ("settings.data_body", "로컬 우선. JSON 백업을 만든 뒤 클라우드로 보낼 수 있습니다."),
            ("settings.cloud", "클라우드 앱 (권장)"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "지원"),
            ("settings.about", "정보"),
            ("settings.donate", "후원"),
            ("settings.feedback", "피드백 (GitHub Issues)"),
            ("more.title", "더보기"),
            ("more.intro", "선택한 차량의 추적 도구. 설정은 톱니바퀴에서."),
            ("more.trackers", "정비 추적"),
            ("more.logs", "기록 및 알림"),
            ("more.quick", "바로가기"),
            ("more.open_settings", "⚙ 설정 열기"),
            ("home.last_service", "최근 정비"),
            ("home.vehicles", "차량"),
            ("home.quick_actions", "빠른 작업"),
            ("home.at_a_glance", "한눈에"),
            ("home.upcoming", "예정 (90일 / 5k)"),
            ("status.language_set", "언어가 저장되었습니다."),
            ("status.language_system", "시스템 언어를 사용합니다."),
        ])
    })
}

fn zh_map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        HashMap::from([
            ("nav.home", "主页"),
            ("nav.cars", "车辆"),
            ("nav.service", "保养"),
            ("nav.tires", "轮胎"),
            ("nav.costs", "费用"),
            ("nav.more", "更多"),
            ("nav.settings", "设置"),
            ("app.title", "FixItGarage"),
            ("common.save", "保存"),
            ("common.delete", "删除"),
            ("common.back", "返回"),
            ("common.back_more", "返回更多"),
            ("common.switch", "切换"),
            ("settings.title", "⚙ 设置"),
            ("settings.intro", "应用偏好。跟踪与工具在“更多”中。"),
            ("settings.appearance", "外观"),
            ("settings.appearance_body", "默认深色。选择会保存在本设备。"),
            ("settings.dark", "深色"),
            ("settings.light", "浅色"),
            ("settings.units", "计量单位"),
            ("settings.units_body", "选择英制或公制。显示时会换算。"),
            ("settings.imperial", "英制"),
            ("settings.metric", "公制"),
            ("settings.language", "语言"),
            ("settings.language_body", "系统默认跟随手机语言。也可只为 FixItGarage 选择语言包。"),
            ("settings.lang_system", "系统默认"),
            ("settings.lang_en", "English"),
            ("settings.lang_es", "Español"),
            ("settings.lang_fr", "Français"),
            ("settings.lang_de", "Deutsch"),
            ("settings.lang_ja", "日本語"),
            ("settings.lang_ko", "한국어"),
            ("settings.lang_zh", "简体中文"),
            ("settings.feature_focus", "功能侧重"),
            ("settings.feature_body", "隐藏仅 DIY 或仅修车厂相关工具。"),
            ("settings.data", "数据与备份"),
            ("settings.data_body", "本地优先。创建 JSON 备份并发送到云应用。"),
            ("settings.cloud", "云应用（推荐）"),
            ("settings.webdav", "Nextcloud / ownCloud / WebDAV"),
            ("settings.support", "支持"),
            ("settings.about", "关于"),
            ("settings.donate", "捐赠"),
            ("settings.feedback", "发送反馈（GitHub Issues）"),
            ("more.title", "更多"),
            ("more.intro", "当前车辆的跟踪与工具。偏好在设置（齿轮）中。"),
            ("more.trackers", "保养跟踪"),
            ("more.logs", "记录与提醒"),
            ("more.quick", "快捷入口"),
            ("more.open_settings", "⚙ 打开设置"),
            ("home.last_service", "上次保养"),
            ("home.vehicles", "车辆"),
            ("home.quick_actions", "快捷操作"),
            ("home.at_a_glance", "总览"),
            ("home.upcoming", "即将到期（90天 / 5k）"),
            ("status.language_set", "语言已保存。"),
            ("status.language_system", "使用系统语言。"),
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
        assert_eq!(resolve_lang(LanguagePref::System, "ja_JP"), Lang::Ja);
        assert_eq!(resolve_lang(LanguagePref::System, "ko-KR"), Lang::Ko);
        assert_eq!(resolve_lang(LanguagePref::System, "zh-Hans-CN"), Lang::Zh);
    }

    #[test]
    fn override_ignores_os() {
        assert_eq!(resolve_lang(LanguagePref::Fr, "en_US"), Lang::Fr);
        assert_eq!(resolve_lang(LanguagePref::Ja, "en_US"), Lang::Ja);
    }

    #[test]
    fn spanish_nav() {
        assert_eq!(t(Lang::Es, "nav.home"), "Inicio");
        assert_eq!(t(Lang::Es, "nav.settings"), "Ajustes");
    }

    #[test]
    fn asian_nav() {
        assert_eq!(t(Lang::Ja, "nav.home"), "ホーム");
        assert_eq!(t(Lang::Ko, "nav.settings"), "설정");
        assert_eq!(t(Lang::Zh, "nav.more"), "更多");
    }

    #[test]
    fn unknown_key_falls_back_to_english_then_key() {
        assert_eq!(t(Lang::Es, "nav.home"), "Inicio");
        // missing key in all packs → key
        assert_eq!(t(Lang::Es, "no.such.key"), "no.such.key");
    }
}
