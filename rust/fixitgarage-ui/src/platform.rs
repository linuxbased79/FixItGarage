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

/// Show a system notification (Android) or log (desktop).
pub fn notify(title: &str, body: &str) {
    #[cfg(target_os = "android")]
    {
        if let Err(e) = notify_android(title, body) {
            eprintln!("notify failed: {e}");
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        eprintln!("NOTIFY: {title} — {body}");
    }
}

#[cfg(target_os = "android")]
fn notify_android(title: &str, body: &str) -> Result<(), String> {
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

    // notify(id, notification)
    let id: jint = 42001;
    env.call_method(
        &nm_obj,
        "notify",
        "(ILandroid/app/Notification;)V",
        &[JValue::Int(id), JValue::Object(&notification)],
    )
    .map_err(|e| format!("notify: {e}"))?;

    Ok(())
}
