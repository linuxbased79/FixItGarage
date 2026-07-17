//! Optional Nextcloud / ownCloud / generic WebDAV backup upload.
//! Uses HTTP Basic auth PUT — works with Nextcloud files path:
//!   https://cloud.example/remote.php/dav/files/USERNAME/FixItGarage/

/// Upload raw bytes to a WebDAV URL with basic auth.
/// `base_url` should end with `/` or a folder path; filename is appended.
pub fn upload_backup(
    base_url: &str,
    username: &str,
    password: &str,
    filename: &str,
    body: &[u8],
) -> Result<String, String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("WebDAV URL is empty".into());
    }
    // Hard-require HTTPS — never send Basic auth over cleartext HTTP.
    if !base.to_ascii_lowercase().starts_with("https://") {
        return Err("WebDAV URL must use https:// (HTTP cleartext is not allowed)".into());
    }
    if username.trim().is_empty() {
        return Err("Username is empty".into());
    }
    if password.is_empty() {
        return Err("Password is empty — save WebDAV password in Settings first".into());
    }
    let url = format!("{base}/{filename}");
    // Basic auth over TLS only (checked above)
    let token = base64_encode(&format!("{}:{}", username.trim(), password));
    let auth = format!("Basic {token}");

    let resp = ureq::put(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &auth)
        .timeout(std::time::Duration::from_secs(60))
        .send_bytes(body)
        .map_err(|e| format!("upload: {e}"))?;

    let status = resp.status();
    if (200..300).contains(&status) || status == 201 || status == 204 {
        Ok(format!("Uploaded to {url} (HTTP {status})"))
    } else {
        Err(format!("Upload failed HTTP {status} for {url}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_http() {
        let err = upload_backup("http://example.com/dav", "u", "p", "f.json", b"{}").unwrap_err();
        assert!(err.to_ascii_lowercase().contains("https"), "{err}");
    }

    #[test]
    fn rejects_empty_password() {
        let err =
            upload_backup("https://example.com/dav", "u", "", "f.json", b"{}").unwrap_err();
        assert!(err.to_ascii_lowercase().contains("password"), "{err}");
    }
}

fn base64_encode(input: &str) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((triple >> 18) & 0x3f) as usize] as char);
        out.push(T[((triple >> 12) & 0x3f) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(T[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(T[(triple & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}
