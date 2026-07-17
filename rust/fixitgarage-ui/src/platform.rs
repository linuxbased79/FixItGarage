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
