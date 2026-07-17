//! Platform helpers (open URL, camera, etc.).

/// Open a URL in the system browser.
pub fn open_url(url: &str) {
    #[cfg(target_os = "android")]
    {
        if let Err(e) = open_url_android(url) {
            eprintln!("open_url android failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(target_os = "android")]
fn open_url_android(url: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;

    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let url_j = env
        .new_string(url)
        .map_err(|e| format!("new_string: {e}"))?;
    let uri_class = env
        .find_class("android/net/Uri")
        .map_err(|e| format!("Uri class: {e}"))?;
    let uri = env
        .call_static_method(
            uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&url_j)],
        )
        .map_err(|e| format!("Uri.parse: {e}"))?
        .l()
        .map_err(|e| format!("Uri obj: {e}"))?;

    let intent_class = env
        .find_class("android/content/Intent")
        .map_err(|e| format!("Intent class: {e}"))?;
    let action = env
        .get_static_field(
            &intent_class,
            "ACTION_VIEW",
            "Ljava/lang/String;",
        )
        .map_err(|e| format!("ACTION_VIEW: {e}"))?
        .l()
        .map_err(|e| format!("ACTION_VIEW obj: {e}"))?;

    let intent = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )
        .map_err(|e| format!("new Intent: {e}"))?;

    // FLAG_ACTIVITY_NEW_TASK = 0x10000000
    let flag = 0x1000_0000i32;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flag)],
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )
    .map_err(|e| format!("startActivity: {e}"))?;

    Ok(())
}

/// Try to open the device camera (Android). Desktop: no-op with path suggestion.
/// Returns a suggested local photo path for the issue log.
pub fn capture_issue_photo_path() -> String {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("fixitgarage")
        .join("photos")
        .join(format!("issue-{stamp}.jpg"));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path_str = path.display().to_string();

    #[cfg(target_os = "android")]
    {
        if let Err(e) = open_camera_android(&path_str) {
            eprintln!("camera intent failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        // Desktop: create a tiny placeholder so the path exists
        if !path.exists() {
            let _ = std::fs::write(
                &path,
                b"FixItGarage photo placeholder - use Android camera for real capture.\n",
            );
        }
    }

    path_str
}

#[cfg(target_os = "android")]
fn open_camera_android(output_path: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let intent_class = env
        .find_class("android/content/Intent")
        .map_err(|e| format!("Intent: {e}"))?;
    let action = env
        .new_string("android.media.action.IMAGE_CAPTURE")
        .map_err(|e| format!("action str: {e}"))?;
    let intent = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|e| format!("new Intent: {e}"))?;

    // FLAG_ACTIVITY_NEW_TASK
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)],
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    // Best-effort: many cameras work with bare IMAGE_CAPTURE without FileProvider.
    // Full EXTRA_OUTPUT needs a content URI (FileProvider) — documented as follow-up.
    let _ = output_path;

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )
    .map_err(|e| format!("startActivity: {e}"))?;

    Ok(())
}

/// Well-known Android package IDs for cloud apps (share-target style backups).
pub const PKG_PROTON_DRIVE: &str = "me.proton.android.drive";
pub const PKG_GOOGLE_DRIVE: &str = "com.google.android.apps.docs";
pub const PKG_DROPBOX: &str = "com.dropbox.android";
/// Microsoft OneDrive
pub const PKG_ONEDRIVE: &str = "com.microsoft.skydrive";

/// Share plain text (CSV/backup body) via the system share sheet when possible.
pub fn share_text(subject: &str, text: &str) {
    share_text_prefer_package(subject, text, None);
}

/// Share to a specific cloud app if installed; otherwise open the full chooser
/// (and try to open the store listing if the package is missing).
pub fn share_text_to_cloud(subject: &str, text: &str, package: &str, app_label: &str) {
    #[cfg(target_os = "android")]
    {
        match share_text_android(subject, text, Some(package)) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("share to {app_label} ({package}) failed: {e}");
                // Fall back to chooser so user can still pick another app
                if let Err(e2) = share_text_android(subject, text, None) {
                    eprintln!("share chooser failed: {e2}");
                    let _ = write_share_fallback(subject, text);
                }
                // Offer install page for the preferred app
                let _ = open_url(&format!("market://details?id={package}"));
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app_label;
        let _ = package;
        share_text_prefer_package(subject, text, None);
    }
}

fn share_text_prefer_package(subject: &str, text: &str, package: Option<&str>) {
    #[cfg(target_os = "android")]
    {
        if let Err(e) = share_text_android(subject, text, package) {
            eprintln!("share_text failed: {e}");
            let _ = write_share_fallback(subject, text);
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = package;
        let path = write_share_fallback(subject, text);
        eprintln!("Share saved to: {}", path.display());
        let _ = std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(std::path::Path::new(".")))
            .spawn();
    }
}

fn write_share_fallback(subject: &str, text: &str) -> std::path::PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("fixitgarage")
        .join("share");
    let _ = std::fs::create_dir_all(&dir);
    let safe: String = subject
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let path = dir.join(format!("{safe}.txt"));
    let _ = std::fs::write(&path, text);
    path
}

#[cfg(target_os = "android")]
fn share_text_android(
    subject: &str,
    text: &str,
    package: Option<&str>,
) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let intent_class = env
        .find_class("android/content/Intent")
        .map_err(|e| format!("Intent: {e}"))?;
    let action_send = env
        .new_string("android.intent.action.SEND")
        .map_err(|e| format!("SEND: {e}"))?;
    let intent = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action_send)],
        )
        .map_err(|e| format!("new Intent: {e}"))?;

    let mime = env
        .new_string("text/plain")
        .map_err(|e| format!("mime: {e}"))?;
    env.call_method(
        &intent,
        "setType",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&mime)],
    )
    .map_err(|e| format!("setType: {e}"))?;

    let extra_text = env
        .new_string("android.intent.extra.TEXT")
        .map_err(|e| format!("extra TEXT key: {e}"))?;
    let text_j = env
        .new_string(text)
        .map_err(|e| format!("text: {e}"))?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&extra_text), JValue::Object(&text_j)],
    )
    .map_err(|e| format!("putExtra TEXT: {e}"))?;

    let extra_sub = env
        .new_string("android.intent.extra.SUBJECT")
        .map_err(|e| format!("extra SUBJECT key: {e}"))?;
    let sub_j = env
        .new_string(subject)
        .map_err(|e| format!("subject: {e}"))?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&extra_sub), JValue::Object(&sub_j)],
    )
    .map_err(|e| format!("putExtra SUBJECT: {e}"))?;

    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)],
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    // Prefer a specific cloud app when requested
    if let Some(pkg) = package {
        let pkg_j = env
            .new_string(pkg)
            .map_err(|e| format!("package str: {e}"))?;
        env.call_method(
            &intent,
            "setPackage",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&pkg_j)],
        )
        .map_err(|e| format!("setPackage: {e}"))?;

        // Direct start — fails if app not installed
        env.call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )
        .map_err(|e| format!("startActivity package {pkg}: {e}"))?;
        return Ok(());
    }

    let chooser_title = env
        .new_string("Share FixItGarage data")
        .map_err(|e| format!("chooser title: {e}"))?;
    let chooser = env
        .call_static_method(
            &intent_class,
            "createChooser",
            "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
            &[JValue::Object(&intent), JValue::Object(&chooser_title)],
        )
        .map_err(|e| format!("createChooser: {e}"))?
        .l()
        .map_err(|e| format!("chooser obj: {e}"))?;

    env.call_method(
        &chooser,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)],
    )
    .map_err(|e| format!("chooser flags: {e}"))?;

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&chooser)],
    )
    .map_err(|e| format!("startActivity: {e}"))?;

    Ok(())
}

/// Show a system notification (Android) or log (desktop). Fixed id 42001.
pub fn notify(title: &str, body: &str) {
    notify_with_id(42001, title, body);
}

/// Notification with a stable integer id (multiple due items can stack).
pub fn notify_with_id(id: i32, title: &str, body: &str) {
    #[cfg(target_os = "android")]
    {
        if let Err(e) = notify_android(id, title, body) {
            eprintln!("notify failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        eprintln!("NOTIFY[{id}]: {title} — {body}");
    }
}

/// Read primary clipboard text (Android) or xclip/wl-paste (desktop best-effort).
pub fn read_clipboard() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        match read_clipboard_android() {
            Ok(s) if !s.trim().is_empty() => Some(s),
            Ok(_) => None,
            Err(e) => {
                eprintln!("clipboard: {e}");
                None
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        for cmd in [
            ("wl-paste", vec![] as Vec<&str>),
            ("xclip", vec!["-selection", "clipboard", "-o"]),
            ("xsel", vec!["--clipboard", "--output"]),
        ] {
            if let Ok(out) = std::process::Command::new(cmd.0).args(&cmd.1).output() {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout).to_string();
                    if !s.trim().is_empty() {
                        return Some(s);
                    }
                }
            }
        }
        None
    }
}

/// Schedule AlarmManager to re-open the app at `trigger_epoch_ms` (RTC_WAKEUP).
/// When the alarm fires, Android launches FixItGarage so launch-time due checks run.
pub fn schedule_app_wake(request_code: i32, trigger_epoch_ms: i64, _label: &str) {
    #[cfg(target_os = "android")]
    {
        if let Err(e) = schedule_app_wake_android(request_code, trigger_epoch_ms) {
            eprintln!("schedule wake failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (request_code, trigger_epoch_ms, _label);
    }
}

/// Persist alarm schedule for BootReceiver (`fig_alarms.json` in app files dir).
/// Format: `[{"code":50001,"due_ms":...,"title":"..."}, ...]`
pub fn write_alarm_schedule(entries: &[(i32, i64, String)]) {
    let mut body = String::from("[");
    for (i, (code, due, title)) in entries.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        let safe: String = title
            .chars()
            .map(|c| match c {
                '"' | '\\' => ' ',
                c if c.is_control() => ' ',
                c => c,
            })
            .collect();
        body.push_str(&format!(
            r#"{{"code":{code},"due_ms":{due},"title":"{safe}"}}"#
        ));
    }
    body.push(']');

    #[cfg(target_os = "android")]
    {
        if let Err(e) = write_alarm_schedule_android(&body) {
            eprintln!("write alarm schedule failed: {e}");
            // Fallback path used by some NativeActivity builds
            let path = android_files_fallback_path().join("fig_alarms.json");
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
            let _ = std::fs::write(&path, &body);
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("fixitgarage")
            .join("fig_alarms.json");
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(&path, body);
    }
}

#[cfg(target_os = "android")]
fn android_files_fallback_path() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("fixitgarage")
}

#[cfg(target_os = "android")]
fn write_alarm_schedule_android(json: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    // File dir = context.getFilesDir()
    let files_dir = env
        .call_method(&context, "getFilesDir", "()Ljava/io/File;", &[])
        .map_err(|e| format!("getFilesDir: {e}"))?
        .l()
        .map_err(|e| format!("filesDir: {e}"))?;
    let path_j = env
        .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("getAbsolutePath: {e}"))?
        .l()
        .map_err(|e| format!("path obj: {e}"))?;
    let jstr: jni::objects::JString = path_j.into();
    let s = env
        .get_string(&jstr)
        .map_err(|e| format!("get_string: {e}"))?;
    let dir: String = s.into();
    let path = std::path::PathBuf::from(dir).join("fig_alarms.json");
    std::fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))?;
    // Touch: also try openFileOutput for consistency with Context
    let _ = env;
    let _ = JValue::Void;
    Ok(())
}

/// Cancel a previously scheduled app-wake alarm.
pub fn cancel_app_wake(request_code: i32) {
    #[cfg(target_os = "android")]
    {
        if let Err(e) = cancel_app_wake_android(request_code) {
            eprintln!("cancel wake failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = request_code;
    }
}

/// Open a text-from-image / OCR helper (Lens / store search).
pub fn open_ocr_helper() {
    #[cfg(target_os = "android")]
    {
        open_url("https://lens.google.com/");
    }
    #[cfg(not(target_os = "android"))]
    {
        eprintln!("OCR helper: paste text from any OCR tool into FixItGarage");
    }
}

#[cfg(target_os = "android")]
fn notify_android(id: i32, title: &str, body: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::sys::jint;
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    // NotificationManager
    let notif_service = env
        .new_string("notification")
        .map_err(|e| format!("svc str: {e}"))?;
    let nm_obj = env
        .call_method(
            &context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&notif_service)],
        )
        .map_err(|e| format!("getSystemService: {e}"))?
        .l()
        .map_err(|e| format!("nm obj: {e}"))?;

    // Channel for API 26+
    let channel_id = env
        .new_string("fixitgarage_reminders")
        .map_err(|e| format!("channel id: {e}"))?;
    let channel_name = env
        .new_string("FixItGarage reminders")
        .map_err(|e| format!("channel name: {e}"))?;
    // IMPORTANCE_DEFAULT = 3
    let channel_class = env
        .find_class("android/app/NotificationChannel")
        .map_err(|e| format!("NotificationChannel: {e}"))?;
    let channel = env
        .new_object(
            &channel_class,
            "(Ljava/lang/String;Ljava/lang/CharSequence;I)V",
            &[
                JValue::Object(&channel_id),
                JValue::Object(&channel_name),
                JValue::Int(3),
            ],
        )
        .map_err(|e| format!("new channel: {e}"))?;
    let _ = env.call_method(
        &nm_obj,
        "createNotificationChannel",
        "(Landroid/app/NotificationChannel;)V",
        &[JValue::Object(&channel)],
    );

    // Builder
    let builder_class = env
        .find_class("android/app/Notification$Builder")
        .map_err(|e| format!("Builder: {e}"))?;
    // Notification.Builder(Context, String) API 26+
    let builder = env
        .new_object(
            &builder_class,
            "(Landroid/content/Context;Ljava/lang/String;)V",
            &[JValue::Object(&context), JValue::Object(&channel_id)],
        )
        .map_err(|e| format!("new Builder: {e}"))?;

    let title_j = env.new_string(title).map_err(|e| format!("title: {e}"))?;
    let body_j = env.new_string(body).map_err(|e| format!("body: {e}"))?;
    env.call_method(
        &builder,
        "setContentTitle",
        "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
        &[JValue::Object(&title_j)],
    )
    .map_err(|e| format!("setContentTitle: {e}"))?;
    env.call_method(
        &builder,
        "setContentText",
        "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
        &[JValue::Object(&body_j)],
    )
    .map_err(|e| format!("setContentText: {e}"))?;
    // Expanded body for longer due lists
    if let Ok(style_class) = env.find_class("android/app/Notification$BigTextStyle") {
        if let Ok(style) = env.new_object(&style_class, "()V", &[]) {
            let _ = env.call_method(
                &style,
                "bigText",
                "(Ljava/lang/CharSequence;)Landroid/app/Notification$BigTextStyle;",
                &[JValue::Object(&body_j)],
            );
            let _ = env.call_method(
                &builder,
                "setStyle",
                "(Landroid/app/Notification$Style;)Landroid/app/Notification$Builder;",
                &[JValue::Object(&style)],
            );
        }
    }
    // android.R.drawable.ic_dialog_info = 17301659
    env.call_method(
        &builder,
        "setSmallIcon",
        "(I)Landroid/app/Notification$Builder;",
        &[JValue::Int(17301659)],
    )
    .map_err(|e| format!("setSmallIcon: {e}"))?;
    env.call_method(
        &builder,
        "setAutoCancel",
        "(Z)Landroid/app/Notification$Builder;",
        &[JValue::Bool(1)],
    )
    .map_err(|e| format!("setAutoCancel: {e}"))?;

    let notification = env
        .call_method(&builder, "build", "()Landroid/app/Notification;", &[])
        .map_err(|e| format!("build: {e}"))?
        .l()
        .map_err(|e| format!("notif: {e}"))?;

    let id: jint = id;
    env.call_method(
        &nm_obj,
        "notify",
        "(ILandroid/app/Notification;)V",
        &[JValue::Int(id), JValue::Object(&notification)],
    )
    .map_err(|e| format!("notify: {e}"))?;

    Ok(())
}

#[cfg(target_os = "android")]
fn read_clipboard_android() -> Result<String, String> {
    use jni::objects::{JObject, JString, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let clip_svc = env
        .new_string("clipboard")
        .map_err(|e| format!("str: {e}"))?;
    let cm = env
        .call_method(
            &context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&clip_svc)],
        )
        .map_err(|e| format!("getSystemService: {e}"))?
        .l()
        .map_err(|e| format!("cm: {e}"))?;

    let has = env
        .call_method(&cm, "hasPrimaryClip", "()Z", &[])
        .map_err(|e| format!("hasPrimaryClip: {e}"))?
        .z()
        .map_err(|e| format!("has z: {e}"))?;
    if !has {
        return Ok(String::new());
    }

    let clip = env
        .call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[])
        .map_err(|e| format!("getPrimaryClip: {e}"))?
        .l()
        .map_err(|e| format!("clip: {e}"))?;
    if clip.is_null() {
        return Ok(String::new());
    }

    let item = env
        .call_method(
            &clip,
            "getItemAt",
            "(I)Landroid/content/ClipData$Item;",
            &[JValue::Int(0)],
        )
        .map_err(|e| format!("getItemAt: {e}"))?
        .l()
        .map_err(|e| format!("item: {e}"))?;

    let coerce = env
        .call_method(
            &item,
            "coerceToText",
            "(Landroid/content/Context;)Ljava/lang/CharSequence;",
            &[JValue::Object(&context)],
        )
        .map_err(|e| format!("coerceToText: {e}"))?
        .l()
        .map_err(|e| format!("text: {e}"))?;
    if coerce.is_null() {
        return Ok(String::new());
    }

    let s = env
        .call_method(&coerce, "toString", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("toString: {e}"))?
        .l()
        .map_err(|e| format!("str obj: {e}"))?;
    let jstr: JString = s.into();
    let rust: String = env
        .get_string(&jstr)
        .map_err(|e| format!("get_string: {e}"))?
        .into();
    Ok(rust)
}

#[cfg(target_os = "android")]
fn schedule_app_wake_android(request_code: i32, trigger_epoch_ms: i64) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let now = chrono::Utc::now().timestamp_millis();
    if trigger_epoch_ms <= now {
        return Ok(());
    }
    if trigger_epoch_ms - now > 365 * 86_400_000 {
        return Ok(());
    }

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let alarm_svc = env
        .new_string("alarm")
        .map_err(|e| format!("alarm str: {e}"))?;
    let am = env
        .call_method(
            &context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&alarm_svc)],
        )
        .map_err(|e| format!("getSystemService alarm: {e}"))?
        .l()
        .map_err(|e| format!("am: {e}"))?;

    let pkg = env
        .call_method(&context, "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("getPackageName: {e}"))?
        .l()
        .map_err(|e| format!("pkg: {e}"))?;

    let pm = env
        .call_method(
            &context,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .map_err(|e| format!("getPackageManager: {e}"))?
        .l()
        .map_err(|e| format!("pm: {e}"))?;

    let launch = env
        .call_method(
            &pm,
            "getLaunchIntentForPackage",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&pkg)],
        )
        .map_err(|e| format!("getLaunchIntentForPackage: {e}"))?
        .l()
        .map_err(|e| format!("launch: {e}"))?;
    if launch.is_null() {
        return Err("no launch intent".into());
    }

    // FLAG_ACTIVITY_NEW_TASK | CLEAR_TOP | SINGLE_TOP
    let flags = 0x1000_0000i32 | 0x0400_0000 | 0x2000_0000;
    env.call_method(
        &launch,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flags)],
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    let extra_key = env
        .new_string("org.fixitgarage.app.DUE_CHECK")
        .map_err(|e| format!("extra key: {e}"))?;
    env.call_method(
        &launch,
        "putExtra",
        "(Ljava/lang/String;Z)Landroid/content/Intent;",
        &[JValue::Object(&extra_key), JValue::Bool(1)],
    )
    .map_err(|e| format!("putExtra: {e}"))?;

    let pi_class = env
        .find_class("android/app/PendingIntent")
        .map_err(|e| format!("PendingIntent: {e}"))?;
    // FLAG_UPDATE_CURRENT | FLAG_IMMUTABLE
    let pi_flags = 0x0800_0000i32 | 0x0400_0000;
    let pi = env
        .call_static_method(
            &pi_class,
            "getActivity",
            "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;",
            &[
                JValue::Object(&context),
                JValue::Int(request_code),
                JValue::Object(&launch),
                JValue::Int(pi_flags),
            ],
        )
        .map_err(|e| format!("getActivity: {e}"))?
        .l()
        .map_err(|e| format!("pi: {e}"))?;

    // RTC_WAKEUP = 0
    let set_res = env.call_method(
        &am,
        "setAndAllowWhileIdle",
        "(IJLandroid/app/PendingIntent;)V",
        &[
            JValue::Int(0),
            JValue::Long(trigger_epoch_ms),
            JValue::Object(&pi),
        ],
    );
    if set_res.is_err() {
        env.call_method(
            &am,
            "set",
            "(IJLandroid/app/PendingIntent;)V",
            &[
                JValue::Int(0),
                JValue::Long(trigger_epoch_ms),
                JValue::Object(&pi),
            ],
        )
        .map_err(|e| format!("AlarmManager.set: {e}"))?;
    }

    Ok(())
}

#[cfg(target_os = "android")]
fn cancel_app_wake_android(request_code: i32) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let alarm_svc = env
        .new_string("alarm")
        .map_err(|e| format!("alarm str: {e}"))?;
    let am = env
        .call_method(
            &context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&alarm_svc)],
        )
        .map_err(|e| format!("getSystemService: {e}"))?
        .l()
        .map_err(|e| format!("am: {e}"))?;

    let pkg = env
        .call_method(&context, "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("pkg: {e}"))?
        .l()
        .map_err(|e| format!("pkg l: {e}"))?;
    let pm = env
        .call_method(
            &context,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .map_err(|e| format!("pm: {e}"))?
        .l()
        .map_err(|e| format!("pm l: {e}"))?;
    let launch = env
        .call_method(
            &pm,
            "getLaunchIntentForPackage",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&pkg)],
        )
        .map_err(|e| format!("launch: {e}"))?
        .l()
        .map_err(|e| format!("launch l: {e}"))?;
    if launch.is_null() {
        return Ok(());
    }

    let pi_class = env
        .find_class("android/app/PendingIntent")
        .map_err(|e| format!("PendingIntent: {e}"))?;
    let pi_flags = 0x0800_0000i32 | 0x0400_0000;
    let pi = env
        .call_static_method(
            &pi_class,
            "getActivity",
            "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;",
            &[
                JValue::Object(&context),
                JValue::Int(request_code),
                JValue::Object(&launch),
                JValue::Int(pi_flags),
            ],
        )
        .map_err(|e| format!("getActivity: {e}"))?
        .l()
        .map_err(|e| format!("pi: {e}"))?;

    env.call_method(
        &am,
        "cancel",
        "(Landroid/app/PendingIntent;)V",
        &[JValue::Object(&pi)],
    )
    .map_err(|e| format!("cancel: {e}"))?;
    Ok(())
}

/// Best-effort system locale tag (e.g. "en_US", "es_MX", "de").
pub fn system_locale() -> String {
    #[cfg(target_os = "android")]
    {
        match system_locale_android() {
            Ok(s) if !s.trim().is_empty() => return s,
            Ok(_) => {}
            Err(e) => eprintln!("system_locale android: {e}"),
        }
    }
    // Desktop / fallback: LANG or LC_ALL
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim();
            if !v.is_empty() && v != "C" && v != "POSIX" {
                // Strip encoding: en_US.UTF-8 → en_US
                let tag = v.split('.').next().unwrap_or(v);
                return tag.to_string();
            }
        }
    }
    "en".into()
}

#[cfg(target_os = "android")]
fn system_locale_android() -> Result<String, String> {
    use jni::objects::{JObject, JString, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let resources = env
        .call_method(
            &context,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )
        .map_err(|e| format!("getResources: {e}"))?
        .l()
        .map_err(|e| format!("resources: {e}"))?;
    let config = env
        .call_method(
            &resources,
            "getConfiguration",
            "()Landroid/content/res/Configuration;",
            &[],
        )
        .map_err(|e| format!("getConfiguration: {e}"))?
        .l()
        .map_err(|e| format!("config: {e}"))?;

    // Prefer LocaleList (API 24+): configuration.getLocales().get(0)
    let from_list = (|| -> Result<String, String> {
        let list = env
            .call_method(&config, "getLocales", "()Landroid/os/LocaleList;", &[])
            .map_err(|e| format!("getLocales: {e}"))?
            .l()
            .map_err(|e| format!("locales: {e}"))?;
        if list.is_null() {
            return Err("null LocaleList".into());
        }
        let locale = env
            .call_method(&list, "get", "(I)Ljava/util/Locale;", &[JValue::Int(0)])
            .map_err(|e| format!("get(0): {e}"))?
            .l()
            .map_err(|e| format!("locale0: {e}"))?;
        if locale.is_null() {
            return Err("null locale".into());
        }
        let tag = env
            .call_method(&locale, "toLanguageTag", "()Ljava/lang/String;", &[])
            .map_err(|e| format!("toLanguageTag: {e}"))?
            .l()
            .map_err(|e| format!("tag obj: {e}"))?;
        let jstr: JString = tag.into();
        let s = env
            .get_string(&jstr)
            .map_err(|e| format!("get_string: {e}"))?;
        Ok(s.into())
    })();
    if let Ok(tag) = from_list {
        if !tag.trim().is_empty() {
            return Ok(tag);
        }
    }

    // Fallback: config.locale (deprecated but widely present)
    let locale_field = env
        .get_field(&config, "locale", "Ljava/util/Locale;")
        .map_err(|e| format!("locale field: {e}"))?
        .l()
        .map_err(|e| format!("locale: {e}"))?;
    if locale_field.is_null() {
        return Ok("en".into());
    }
    let tag = env
        .call_method(&locale_field, "toString", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("toString: {e}"))?
        .l()
        .map_err(|e| format!("tag: {e}"))?;
    let jstr: JString = tag.into();
    let s = env
        .get_string(&jstr)
        .map_err(|e| format!("get_string: {e}"))?;
    Ok(s.into())
}
