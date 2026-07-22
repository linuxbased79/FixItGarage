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

/// Share a local file (e.g. seller PDF) via ACTION_SEND when possible.
pub fn share_file(subject: &str, path: &str, mime: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        return share_file_android(subject, path, mime);
    }
    #[cfg(not(target_os = "android"))]
    {
        let p = std::path::Path::new(path);
        if !p.is_file() {
            return Err(format!("file missing: {path}"));
        }
        let _ = subject;
        let _ = mime;
        let _ = std::process::Command::new("xdg-open").arg(p).spawn();
        Ok(())
    }
}

#[cfg(target_os = "android")]
fn share_file_android(subject: &str, path: &str, mime: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let bytes = std::fs::read(path).map_err(|e| format!("read file: {e}"))?;
    if bytes.is_empty() {
        return Err("empty file".into());
    }

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    // Publish into MediaStore Downloads/Documents for a shareable content URI
    let resolver = env
        .call_method(
            &context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .map_err(|e| format!("resolver: {e}"))?
        .l()
        .map_err(|e| format!("resolver l: {e}"))?;

    let cv_class = env
        .find_class("android/content/ContentValues")
        .map_err(|e| format!("CV: {e}"))?;
    let cv = env
        .new_object(&cv_class, "()V", &[])
        .map_err(|e| format!("new CV: {e}"))?;

    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fixitgarage-packet.pdf");
    for (k, v) in [
        ("_display_name", name),
        ("mime_type", mime),
        ("relative_path", "Download/FixItGarage"),
    ] {
        let kj = env.new_string(k).map_err(|e| format!("k: {e}"))?;
        let vj = env.new_string(v).map_err(|e| format!("v: {e}"))?;
        env.call_method(
            &cv,
            "put",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&kj), JValue::Object(&vj)],
        )
        .map_err(|e| format!("put: {e}"))?;
    }

    // MediaStore.Downloads.EXTERNAL_CONTENT_URI (API 29+) or Files
    let media_class = env
        .find_class("android/provider/MediaStore$Downloads")
        .or_else(|_| env.find_class("android/provider/MediaStore$Files"))
        .map_err(|e| format!("MediaStore: {e}"))?;
    let ext_uri = env
        .get_static_field(
            &media_class,
            "EXTERNAL_CONTENT_URI",
            "Landroid/net/Uri;",
        )
        .map_err(|e| format!("EXTERNAL: {e}"))?
        .l()
        .map_err(|e| format!("uri: {e}"))?;

    let inserted = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&ext_uri), JValue::Object(&cv)],
        )
        .map_err(|e| format!("insert: {e}"))?
        .l()
        .map_err(|e| format!("inserted: {e}"))?;
    if inserted.is_null() {
        return Err("MediaStore insert null — try Share seller summary (text)".into());
    }

    let out_stream = env
        .call_method(
            &resolver,
            "openOutputStream",
            "(Landroid/net/Uri;)Ljava/io/OutputStream;",
            &[JValue::Object(&inserted)],
        )
        .map_err(|e| format!("openOutputStream: {e}"))?
        .l()
        .map_err(|e| format!("ostream: {e}"))?;
    if out_stream.is_null() {
        return Err("openOutputStream null".into());
    }
    let jbytes = env
        .byte_array_from_slice(&bytes)
        .map_err(|e| format!("jbytes: {e}"))?;
    env.call_method(
        &out_stream,
        "write",
        "([B)V",
        &[JValue::Object(jbytes.as_ref())],
    )
    .map_err(|e| format!("write: {e}"))?;
    let _ = env.call_method(&out_stream, "flush", "()V", &[]);
    let _ = env.call_method(&out_stream, "close", "()V", &[]);

    // ACTION_SEND with stream
    let intent_class = env
        .find_class("android/content/Intent")
        .map_err(|e| format!("Intent: {e}"))?;
    let action = env
        .new_string("android.intent.action.SEND")
        .map_err(|e| format!("SEND: {e}"))?;
    let intent = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|e| format!("new Intent: {e}"))?;
    let mime_j = env.new_string(mime).map_err(|e| format!("mime: {e}"))?;
    env.call_method(
        &intent,
        "setType",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&mime_j)],
    )
    .map_err(|e| format!("setType: {e}"))?;
    let stream_key = env
        .new_string("android.intent.extra.STREAM")
        .map_err(|e| format!("STREAM: {e}"))?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Landroid/os/Parcelable;)Landroid/content/Intent;",
        &[JValue::Object(&stream_key), JValue::Object(&inserted)],
    )
    .map_err(|e| format!("putExtra STREAM: {e}"))?;
    let sub_key = env
        .new_string("android.intent.extra.SUBJECT")
        .map_err(|e| format!("SUBJECT: {e}"))?;
    let sub_j = env.new_string(subject).map_err(|e| format!("sub: {e}"))?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&sub_key), JValue::Object(&sub_j)],
    )
    .map_err(|e| format!("putExtra SUBJECT: {e}"))?;
    // FLAG_GRANT_READ_URI_PERMISSION | NEW_TASK
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x0000_0001 | 0x1000_0000)],
    )
    .map_err(|e| format!("flags: {e}"))?;

    let title = env
        .new_string("Share maintenance packet")
        .map_err(|e| format!("title: {e}"))?;
    let chooser = env
        .call_static_method(
            &intent_class,
            "createChooser",
            "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
            &[JValue::Object(&intent), JValue::Object(&title)],
        )
        .map_err(|e| format!("chooser: {e}"))?
        .l()
        .map_err(|e| format!("chooser l: {e}"))?;
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

/// Files written by ShareReceiveActivity / native OCR capture (app files dir).
pub const OCR_TEXT_FILE: &str = "fig_ocr_text.txt";
pub const OCR_IMAGE_FILE: &str = "fig_ocr_image.jpg";
#[allow(dead_code)] // written/read on Android MediaStore path
pub const OCR_URI_FILE: &str = "fig_ocr_uri.txt";
pub const OCR_TARGET_FILE: &str = "fig_ocr_target.txt";

/// F-Droid / Play OCR packages (Graphene-friendly first).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
const OCR_APP_PACKAGES: &[&str] = &[
    "com.renard.ocr",                          // Text Fairy (F-Droid)
    "com.google.android.apps.lens",            // Google Lens
    "com.google.ar.lens",
    "com.google.android.googlequicksearchbox",
    "com.microsoft.office.officehubrow",
    "com.microsoft.office.officelens",
];

/// Open a text-from-image / OCR helper (receipt form target).
/// Prefers an installed OCR app (GrapheneOS / F-Droid friendly), then store
/// search, then Lens web as last resort.
pub fn open_ocr_helper() {
    set_ocr_target("receipt");
    open_ocr_helper_inner();
}

/// Remember which form should consume the next shared OCR text (`receipt` / `tire`).
pub fn set_ocr_target(target: &str) {
    if let Some(path) = ocr_files_dir().map(|d| d.join(OCR_TARGET_FILE)) {
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(path, target);
    }
}

/// Read OCR target (`receipt` default).
pub fn ocr_target() -> String {
    ocr_files_dir()
        .and_then(|d| std::fs::read_to_string(d.join(OCR_TARGET_FILE)).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "receipt".into())
}

/// Take pending shared/captured OCR text (consumes file). Also checks clipboard
/// as a soft fallback when the share target was not used.
pub fn take_pending_ocr_text() -> Option<String> {
    if let Some(dir) = ocr_files_dir() {
        let path = dir.join(OCR_TEXT_FILE);
        if let Ok(text) = std::fs::read_to_string(&path) {
            let _ = std::fs::remove_file(&path);
            let t = text.trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

/// Path to pending OCR image if present (does not delete).
pub fn pending_ocr_image_path() -> Option<String> {
    let dir = ocr_files_dir()?;
    let path = dir.join(OCR_IMAGE_FILE);
    if path.is_file() {
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > 32 {
                return Some(path.display().to_string());
            }
        }
    }
    // Also try content URI → copy into image file
    #[cfg(target_os = "android")]
    {
        if let Ok(Some(p)) = finalize_pending_camera_uri() {
            return Some(p);
        }
    }
    None
}

/// Capture a receipt photo into MediaStore (Android) and remember URI for OCR share.
/// Returns a display path / URI string.
pub fn capture_receipt_for_ocr() -> String {
    set_ocr_target("receipt");
    #[cfg(target_os = "android")]
    {
        match capture_for_ocr_android() {
            Ok(uri) => {
                if let Some(dir) = ocr_files_dir() {
                    let _ = std::fs::write(dir.join(OCR_URI_FILE), &uri);
                }
                return uri;
            }
            Err(e) => {
                eprintln!("capture for OCR: {e}");
                // Fall back to generic camera path
            }
        }
    }
    capture_issue_photo_path()
}

/// Send the pending OCR image (or last capture URI) to an installed OCR app via
/// ACTION_SEND. User can then Share text back to FixItGarage or copy + Paste & fill.
pub fn send_pending_image_to_ocr() -> Result<(), String> {
    set_ocr_target("receipt");
    #[cfg(target_os = "android")]
    {
        // Prefer content URI (grantable); else try file path via MediaStore re-insert
        if let Some(dir) = ocr_files_dir() {
            let uri_path = dir.join(OCR_URI_FILE);
            if let Ok(uri) = std::fs::read_to_string(&uri_path) {
                let uri = uri.trim();
                if !uri.is_empty() {
                    return send_image_uri_to_ocr_android(uri);
                }
            }
            let img = dir.join(OCR_IMAGE_FILE);
            if img.is_file() {
                // Re-publish app file into MediaStore so OCR apps can read it
                match publish_file_to_mediastore_android(&img.display().to_string()) {
                    Ok(uri) => {
                        let _ = std::fs::write(&uri_path, &uri);
                        return send_image_uri_to_ocr_android(&uri);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Err("No receipt photo yet — capture first.".into())
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("Send photo to OCR is available on Android.".into())
    }
}

/// Launch OCR helper targeted at tire receipt form.
pub fn open_ocr_helper_for_tire() {
    set_ocr_target("tire");
    open_ocr_helper_inner();
}

fn open_ocr_helper_inner() {
    #[cfg(target_os = "android")]
    {
        for pkg in OCR_APP_PACKAGES {
            if try_launch_package(pkg) {
                return;
            }
        }
        open_url("https://f-droid.org/packages/com.renard.ocr/");
        open_url("market://search?q=OCR%20text%20scanner&c=apps");
        open_url("https://lens.google.com/");
    }
    #[cfg(not(target_os = "android"))]
    {
        eprintln!("OCR helper: paste text from any OCR tool into FixItGarage");
    }
}

fn ocr_files_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    {
        if let Ok(p) = android_files_dir() {
            return Some(p);
        }
        return Some(android_files_fallback_path());
    }
    #[cfg(not(target_os = "android"))]
    {
        let p = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("fixitgarage");
        let _ = std::fs::create_dir_all(&p);
        Some(p)
    }
}

/// Public wrapper for app files dir (OCR models, share drops).
pub fn android_files_dir_public() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        return android_files_dir();
    }
    #[cfg(not(target_os = "android"))]
    {
        Ok(dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("fixitgarage"))
    }
}

/// Extract an APK asset (e.g. `models/text-detection.rten`) into `dest`.
pub fn extract_asset_to_file(asset_path: &str, dest: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        return extract_asset_to_file_android(asset_path, dest);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (asset_path, dest);
        Err("assets only on Android".into())
    }
}

#[cfg(target_os = "android")]
fn extract_asset_to_file_android(asset_path: &str, dest: &std::path::Path) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    if dest.is_file() {
        if let Ok(m) = std::fs::metadata(dest) {
            if m.len() > 1000 {
                return Ok(());
            }
        }
    }
    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let assets = env
        .call_method(
            &context,
            "getAssets",
            "()Landroid/content/res/AssetManager;",
            &[],
        )
        .map_err(|e| format!("getAssets: {e}"))?
        .l()
        .map_err(|e| format!("assets: {e}"))?;

    let path_j = env
        .new_string(asset_path)
        .map_err(|e| format!("path: {e}"))?;
    let input = env
        .call_method(
            &assets,
            "open",
            "(Ljava/lang/String;)Ljava/io/InputStream;",
            &[JValue::Object(&path_j)],
        )
        .map_err(|e| format!("assets.open {asset_path}: {e}"))?
        .l()
        .map_err(|e| format!("stream: {e}"))?;
    if input.is_null() {
        return Err(format!("asset missing: {asset_path}"));
    }

    let mut data: Vec<u8> = Vec::new();
    let buf = env
        .new_byte_array(8192)
        .map_err(|e| format!("byte array: {e}"))?;
    loop {
        let n = env
            .call_method(
                &input,
                "read",
                "([B)I",
                &[JValue::Object(buf.as_ref())],
            )
            .map_err(|e| format!("read: {e}"))?
            .i()
            .map_err(|e| format!("read i: {e}"))?;
        if n <= 0 {
            break;
        }
        let chunk = env
            .convert_byte_array(&buf)
            .map_err(|e| format!("convert: {e}"))?;
        data.extend_from_slice(&chunk[..n as usize]);
        if data.len() > 40 * 1024 * 1024 {
            break;
        }
    }
    let _ = env.call_method(&input, "close", "()V", &[]);
    if data.is_empty() {
        return Err(format!("empty asset: {asset_path}"));
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(dest, data).map_err(|e| format!("write asset: {e}"))?;
    Ok(())
}

#[cfg(target_os = "android")]
fn android_files_dir() -> Result<std::path::PathBuf, String> {
    use jni::objects::JObject;
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };
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
    Ok(std::path::PathBuf::from(Into::<String>::into(s)))
}

/// App-private data directory for `state.json` and other durable files.
/// On Android this prefers `Context.getFilesDir()` (survives restarts).
/// The resolved path is cached so load and save never diverge.
pub fn app_data_dir() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static DIR: OnceLock<std::path::PathBuf> = OnceLock::new();
    DIR.get_or_init(resolve_app_data_dir).clone()
}

/// All places we might have written state (for migration / recovery).
pub fn app_data_dir_candidates() -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    {
        let mut out = Vec::new();
        if let Ok(p) = android_files_dir() {
            out.push(p);
        }
        // Standard package private storage (works even if JNI probe fails).
        out.push(std::path::PathBuf::from(
            "/data/user/0/org.fixitgarage.app/files",
        ));
        out.push(std::path::PathBuf::from(
            "/data/data/org.fixitgarage.app/files",
        ));
        // Legacy dirs crate / XDG-style (often wrong on Android, but may hold old data).
        out.push(android_files_fallback_path());
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                out.push(std::path::PathBuf::from(&home).join("files"));
                out.push(std::path::PathBuf::from(&home).join(".local/share/fixitgarage"));
            }
        }
        // Dedup while preserving order
        let mut seen = std::collections::HashSet::new();
        out.retain(|p| seen.insert(p.clone()));
        out
    }
    #[cfg(not(target_os = "android"))]
    {
        vec![dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("fixitgarage")]
    }
}

fn probe_writable(dir: &std::path::Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".fig_write_probe");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn resolve_app_data_dir() -> std::path::PathBuf {
    for c in app_data_dir_candidates() {
        if probe_writable(&c) {
            eprintln!("FixItGarage: app data dir = {}", c.display());
            return c;
        }
    }
    // Last resort — still try package files dir even if probe failed.
    #[cfg(target_os = "android")]
    {
        let p = std::path::PathBuf::from("/data/user/0/org.fixitgarage.app/files");
        let _ = std::fs::create_dir_all(&p);
        eprintln!("FixItGarage: app data dir fallback = {}", p.display());
        return p;
    }
    #[cfg(not(target_os = "android"))]
    {
        let p = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("fixitgarage");
        let _ = std::fs::create_dir_all(&p);
        p
    }
}

/// Launch an app by package name if installed. Returns true on success.
#[cfg(target_os = "android")]
fn try_launch_package(package: &str) -> bool {
    match try_launch_package_android(package) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("launch {package}: {e}");
            false
        }
    }
}

#[cfg(target_os = "android")]
fn try_launch_package_android(package: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

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

    let pkg = env
        .new_string(package)
        .map_err(|e| format!("pkg str: {e}"))?;
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
        return Err("not installed".into());
    }

    env.call_method(
        &launch,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)], // FLAG_ACTIVITY_NEW_TASK
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&launch)],
    )
    .map_err(|e| format!("startActivity: {e}"))?;
    Ok(())
}

#[cfg(target_os = "android")]
fn capture_for_ocr_android() -> Result<String, String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    // ContentResolver
    let resolver = env
        .call_method(
            &context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .map_err(|e| format!("getContentResolver: {e}"))?
        .l()
        .map_err(|e| format!("resolver: {e}"))?;

    // ContentValues
    let cv_class = env
        .find_class("android/content/ContentValues")
        .map_err(|e| format!("ContentValues: {e}"))?;
    let cv = env
        .new_object(&cv_class, "()V", &[])
        .map_err(|e| format!("new ContentValues: {e}"))?;

    let put_string = |env: &mut jni::JNIEnv, cv: &JObject, key: &str, val: &str| -> Result<(), String> {
        let k = env.new_string(key).map_err(|e| format!("key: {e}"))?;
        let v = env.new_string(val).map_err(|e| format!("val: {e}"))?;
        env.call_method(
            cv,
            "put",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&k), JValue::Object(&v)],
        )
        .map_err(|e| format!("put: {e}"))?;
        Ok(())
    };

    let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    // MediaStore.Images.Media.DISPLAY_NAME / MIME_TYPE / RELATIVE_PATH
    put_string(&mut env, &cv, "_display_name", &format!("fixitgarage_receipt_{stamp}.jpg"))?;
    put_string(&mut env, &cv, "mime_type", "image/jpeg")?;
    put_string(&mut env, &cv, "relative_path", "Pictures/FixItGarage")?;

    // MediaStore.Images.Media.EXTERNAL_CONTENT_URI
    let media_class = env
        .find_class("android/provider/MediaStore$Images$Media")
        .map_err(|e| format!("MediaStore.Images.Media: {e}"))?;
    let ext_uri = env
        .get_static_field(
            media_class,
            "EXTERNAL_CONTENT_URI",
            "Landroid/net/Uri;",
        )
        .map_err(|e| format!("EXTERNAL_CONTENT_URI: {e}"))?
        .l()
        .map_err(|e| format!("uri: {e}"))?;

    let inserted = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&ext_uri), JValue::Object(&cv)],
        )
        .map_err(|e| format!("insert: {e}"))?
        .l()
        .map_err(|e| format!("inserted: {e}"))?;
    if inserted.is_null() {
        return Err("MediaStore insert returned null".into());
    }

    let uri_str_j = env
        .call_method(&inserted, "toString", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("uri toString: {e}"))?
        .l()
        .map_err(|e| format!("uri str: {e}"))?;
    let jstr: jni::objects::JString = uri_str_j.into();
    let uri_string: String = env
        .get_string(&jstr)
        .map_err(|e| format!("get_string: {e}"))?
        .into();

    // IMAGE_CAPTURE with EXTRA_OUTPUT
    let intent_class = env
        .find_class("android/content/Intent")
        .map_err(|e| format!("Intent: {e}"))?;
    let action = env
        .new_string("android.media.action.IMAGE_CAPTURE")
        .map_err(|e| format!("action: {e}"))?;
    let intent = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|e| format!("new Intent: {e}"))?;

    let extra_output = env
        .new_string("output")
        .map_err(|e| format!("extra: {e}"))?;
    // MediaStore.EXTRA_OUTPUT = "output"
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Landroid/os/Parcelable;)Landroid/content/Intent;",
        &[JValue::Object(&extra_output), JValue::Object(&inserted)],
    )
    .map_err(|e| format!("putExtra OUTPUT: {e}"))?;

    // FLAG_GRANT_WRITE_URI_PERMISSION | FLAG_GRANT_READ_URI_PERMISSION | NEW_TASK
    let flags = 0x0000_0002i32 | 0x0000_0001 | 0x1000_0000;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flags)],
    )
    .map_err(|e| format!("addFlags: {e}"))?;

    // Grant URI perms to camera packages (best-effort)
    let _ = env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x0000_0040)], // FLAG_GRANT_PERSISTABLE? skip
    );

    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )
    .map_err(|e| format!("startActivity: {e}"))?;

    Ok(uri_string)
}

/// If a pending MediaStore URI has content, copy into fig_ocr_image.jpg.
#[cfg(target_os = "android")]
fn finalize_pending_camera_uri() -> Result<Option<String>, String> {
    let dir = android_files_dir().unwrap_or_else(|_| android_files_fallback_path());
    let uri_file = dir.join(OCR_URI_FILE);
    let uri = match std::fs::read_to_string(&uri_file) {
        Ok(u) => u.trim().to_string(),
        Err(_) => return Ok(None),
    };
    if uri.is_empty() {
        return Ok(None);
    }
    let out = dir.join(OCR_IMAGE_FILE);
    // Skip if already have a recent image
    if out.is_file() {
        if let Ok(meta) = std::fs::metadata(&out) {
            if meta.len() > 1000 {
                return Ok(Some(out.display().to_string()));
            }
        }
    }
    copy_content_uri_to_file_android(&uri, &out.display().to_string())?;
    if out.is_file() {
        Ok(Some(out.display().to_string()))
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "android")]
fn copy_content_uri_to_file_android(uri: &str, dest: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let resolver = env
        .call_method(
            &context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .map_err(|e| format!("resolver: {e}"))?
        .l()
        .map_err(|e| format!("resolver l: {e}"))?;

    let uri_class = env
        .find_class("android/net/Uri")
        .map_err(|e| format!("Uri: {e}"))?;
    let uri_j = env
        .new_string(uri)
        .map_err(|e| format!("uri str: {e}"))?;
    let uri_obj = env
        .call_static_method(
            uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&uri_j)],
        )
        .map_err(|e| format!("Uri.parse: {e}"))?
        .l()
        .map_err(|e| format!("uri obj: {e}"))?;

    let input = env
        .call_method(
            &resolver,
            "openInputStream",
            "(Landroid/net/Uri;)Ljava/io/InputStream;",
            &[JValue::Object(&uri_obj)],
        )
        .map_err(|e| format!("openInputStream: {e}"))?
        .l()
        .map_err(|e| format!("stream: {e}"))?;
    if input.is_null() {
        return Err("openInputStream null (photo not ready?)".into());
    }

    // Read all bytes via available/read loop into Rust Vec, then write file
    let mut data: Vec<u8> = Vec::new();
    let buf = env
        .new_byte_array(8192)
        .map_err(|e| format!("byte array: {e}"))?;
    loop {
        let n = env
            .call_method(
                &input,
                "read",
                "([B)I",
                &[JValue::Object(buf.as_ref())],
            )
            .map_err(|e| format!("read: {e}"))?
            .i()
            .map_err(|e| format!("read i: {e}"))?;
        if n <= 0 {
            break;
        }
        let chunk = env
            .convert_byte_array(&buf)
            .map_err(|e| format!("convert: {e}"))?;
        data.extend_from_slice(&chunk[..n as usize]);
        if data.len() > 25 * 1024 * 1024 {
            break;
        }
    }
    let _ = env.call_method(&input, "close", "()V", &[]);
    if data.is_empty() {
        return Err("empty image stream".into());
    }
    if let Some(parent) = std::path::Path::new(dest).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(dest, data).map_err(|e| format!("write dest: {e}"))?;
    Ok(())
}

#[cfg(target_os = "android")]
fn publish_file_to_mediastore_android(path: &str) -> Result<String, String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| format!("JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let bytes = std::fs::read(path).map_err(|e| format!("read file: {e}"))?;
    if bytes.is_empty() {
        return Err("empty image file".into());
    }

    let resolver = env
        .call_method(
            &context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .map_err(|e| format!("resolver: {e}"))?
        .l()
        .map_err(|e| format!("resolver l: {e}"))?;

    let cv_class = env
        .find_class("android/content/ContentValues")
        .map_err(|e| format!("CV: {e}"))?;
    let cv = env
        .new_object(&cv_class, "()V", &[])
        .map_err(|e| format!("new CV: {e}"))?;
    let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let name = format!("fixitgarage_ocr_{stamp}.jpg");
    for (k, v) in [
        ("_display_name", name.as_str()),
        ("mime_type", "image/jpeg"),
        ("relative_path", "Pictures/FixItGarage"),
    ] {
        let kj = env.new_string(k).map_err(|e| format!("k: {e}"))?;
        let vj = env.new_string(v).map_err(|e| format!("v: {e}"))?;
        env.call_method(
            &cv,
            "put",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&kj), JValue::Object(&vj)],
        )
        .map_err(|e| format!("put: {e}"))?;
    }

    let media_class = env
        .find_class("android/provider/MediaStore$Images$Media")
        .map_err(|e| format!("Media: {e}"))?;
    let ext_uri = env
        .get_static_field(
            media_class,
            "EXTERNAL_CONTENT_URI",
            "Landroid/net/Uri;",
        )
        .map_err(|e| format!("EXTERNAL: {e}"))?
        .l()
        .map_err(|e| format!("uri: {e}"))?;

    let inserted = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&ext_uri), JValue::Object(&cv)],
        )
        .map_err(|e| format!("insert: {e}"))?
        .l()
        .map_err(|e| format!("inserted: {e}"))?;
    if inserted.is_null() {
        return Err("insert null".into());
    }

    let out_stream = env
        .call_method(
            &resolver,
            "openOutputStream",
            "(Landroid/net/Uri;)Ljava/io/OutputStream;",
            &[JValue::Object(&inserted)],
        )
        .map_err(|e| format!("openOutputStream: {e}"))?
        .l()
        .map_err(|e| format!("ostream: {e}"))?;
    if out_stream.is_null() {
        return Err("openOutputStream null".into());
    }

    let jbytes = env
        .byte_array_from_slice(&bytes)
        .map_err(|e| format!("jbytes: {e}"))?;
    env.call_method(
        &out_stream,
        "write",
        "([B)V",
        &[JValue::Object(jbytes.as_ref())],
    )
    .map_err(|e| format!("write: {e}"))?;
    let _ = env.call_method(&out_stream, "flush", "()V", &[]);
    let _ = env.call_method(&out_stream, "close", "()V", &[]);

    let uri_str_j = env
        .call_method(&inserted, "toString", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("toString: {e}"))?
        .l()
        .map_err(|e| format!("uri str: {e}"))?;
    let jstr: jni::objects::JString = uri_str_j.into();
    let s = env
        .get_string(&jstr)
        .map_err(|e| format!("get_string: {e}"))?;
    Ok(s.into())
}

#[cfg(target_os = "android")]
fn send_image_uri_to_ocr_android(uri: &str) -> Result<(), String> {
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
        .new_string("android.intent.action.SEND")
        .map_err(|e| format!("SEND: {e}"))?;
    let intent = env
        .new_object(
            &intent_class,
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|e| format!("new Intent: {e}"))?;

    let mime = env
        .new_string("image/jpeg")
        .map_err(|e| format!("mime: {e}"))?;
    env.call_method(
        &intent,
        "setType",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&mime)],
    )
    .map_err(|e| format!("setType: {e}"))?;

    let uri_class = env
        .find_class("android/net/Uri")
        .map_err(|e| format!("Uri: {e}"))?;
    let uri_j = env.new_string(uri).map_err(|e| format!("uri: {e}"))?;
    let uri_obj = env
        .call_static_method(
            uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&uri_j)],
        )
        .map_err(|e| format!("parse: {e}"))?
        .l()
        .map_err(|e| format!("uri obj: {e}"))?;

    let stream_key = env
        .new_string("android.intent.extra.STREAM")
        .map_err(|e| format!("STREAM: {e}"))?;
    env.call_method(
        &intent,
        "putExtra",
        "(Ljava/lang/String;Landroid/os/Parcelable;)Landroid/content/Intent;",
        &[JValue::Object(&stream_key), JValue::Object(&uri_obj)],
    )
    .map_err(|e| format!("putExtra STREAM: {e}"))?;

    // FLAG_GRANT_READ_URI_PERMISSION | NEW_TASK
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x0000_0001 | 0x1000_0000)],
    )
    .map_err(|e| format!("flags: {e}"))?;

    // Prefer a known OCR package if installed
    for pkg in OCR_APP_PACKAGES {
        let pkg_j = env.new_string(*pkg).map_err(|e| format!("pkg: {e}"))?;
        // clone intent for setPackage trial
        let trial = env
            .call_method(
                &intent,
                "setPackage",
                "(Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&pkg_j)],
            )
            .ok();
        let _ = trial;
        // Check resolve
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
        // Prefer simple: setPackage and try start; clear package on failure
        match env.call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        ) {
            Ok(_) => return Ok(()),
            Err(_) => {
                // clear package
                let null_str: JObject = JObject::null();
                let _ = env.call_method(
                    &intent,
                    "setPackage",
                    "(Ljava/lang/String;)Landroid/content/Intent;",
                    &[JValue::Object(&null_str)],
                );
                let _ = pm;
            }
        }
    }

    // Chooser fallback
    let title = env
        .new_string("OCR this receipt")
        .map_err(|e| format!("title: {e}"))?;
    let chooser = env
        .call_static_method(
            &intent_class,
            "createChooser",
            "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
            &[JValue::Object(&intent), JValue::Object(&title)],
        )
        .map_err(|e| format!("chooser: {e}"))?
        .l()
        .map_err(|e| format!("chooser l: {e}"))?;
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
    .map_err(|e| format!("start chooser: {e}"))?;
    Ok(())
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

/// System bar insets in density-independent pixels (top status / bottom nav).
/// Used so the app chrome clears the 3-button / gesture navigation bar.
pub fn system_safe_area_dp() -> (f32, f32) {
    #[cfg(target_os = "android")]
    {
        match system_safe_area_dp_android() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("system_safe_area_dp: {e}");
                // Pixel-class 3-button nav is typically ~48dp; status ~24–28dp
                (28.0, 48.0)
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        (0.0, 0.0)
    }
}

#[cfg(target_os = "android")]
fn system_safe_area_dp_android() -> Result<(f32, f32), String> {
    use jni::objects::{JObject, JValue};
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

    let metrics = env
        .call_method(
            &resources,
            "getDisplayMetrics",
            "()Landroid/util/DisplayMetrics;",
            &[],
        )
        .map_err(|e| format!("getDisplayMetrics: {e}"))?
        .l()
        .map_err(|e| format!("metrics: {e}"))?;

    let density = env
        .get_field(&metrics, "density", "F")
        .map_err(|e| format!("density: {e}"))?
        .f()
        .map_err(|e| format!("density f: {e}"))?;
    let density = if density > 0.1 { density } else { 3.0 };

    let mut dimen_px = |name: &str| -> Result<i32, String> {
        let name_j = env
            .new_string(name)
            .map_err(|e| format!("name: {e}"))?;
        let def_type = env
            .new_string("dimen")
            .map_err(|e| format!("dimen: {e}"))?;
        let def_pkg = env
            .new_string("android")
            .map_err(|e| format!("android: {e}"))?;
        let id = env
            .call_method(
                &resources,
                "getIdentifier",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                &[
                    JValue::Object(&name_j),
                    JValue::Object(&def_type),
                    JValue::Object(&def_pkg),
                ],
            )
            .map_err(|e| format!("getIdentifier {name}: {e}"))?
            .i()
            .map_err(|e| format!("id i: {e}"))?;
        if id == 0 {
            return Ok(0);
        }
        let px = env
            .call_method(
                &resources,
                "getDimensionPixelSize",
                "(I)I",
                &[JValue::Int(id)],
            )
            .map_err(|e| format!("getDimensionPixelSize {name}: {e}"))?
            .i()
            .map_err(|e| format!("px i: {e}"))?;
        Ok(px)
    };

    let status_px = dimen_px("status_bar_height").unwrap_or(0);
    let mut nav_px = dimen_px("navigation_bar_height").unwrap_or(0);

    // Some gesture-nav devices report 0 here while a thin bar still steals taps.
    // Enforce a minimum bottom inset on phones so the last tab stays tappable.
    let min_nav_px = (48.0 * density).round() as i32;
    if nav_px < min_nav_px {
        // Prefer real value when present (gesture bar ~16–24dp); else use 48dp for 3-button.
        if nav_px <= 0 {
            nav_px = min_nav_px;
        }
    }

    let top_dp = (status_px as f32 / density).max(0.0);
    let bottom_dp = (nav_px as f32 / density).max(24.0);
    Ok((top_dp, bottom_dp))
}
