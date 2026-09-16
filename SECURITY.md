# Security notes — Motor Noter

Last reviewed with hardening in **0.2.43**. Display name **Motor Noter**; package `org.fixitgarage.app`.

## Design

- Local-first vehicle notes; no Motor Noter account or analytics backend  
- Network is optional and user-initiated (recalls, WebDAV, share, open links)  
- No Google Play Services / ML Kit dependency for core features  

## Hardening (0.2.24+)

| Control | Behavior |
|---------|----------|
| Shared JSON backups | **Never** include WebDAV password |
| Restore from backup | Discards any password in the file (re-enter) |
| WebDAV URL | Must be `https://` — cleartext HTTP rejected |
| WebDAV password UI | Not re-filled after save; blank keeps existing |
| Android Auto Backup | `allowBackup=false` + data extraction rules exclude app data |
| Share target | `SEND` text/image only (no broad `VIEW`/`BROWSABLE`) |
| Release builds | `debuggable=false` |

## Residual risks

- On-device `state.json` in `getFilesDir()` still holds WebDAV password (needed for upload without re-entry). Not copied to SharedPreferences or external mirrors. Readable on a rooted device.  
- Camera still grants the capture URI to OEM camera packages briefly so the photo writes; grants and the public MediaStore row are removed after copy.  
- Shared backups / seller PDFs / CSV still contain **maintenance data** (by design).  
- VIN is sent to NHTSA when you run a recall check.  
- Sideload APKs may be debug-signed; use Play/F-Droid production signing for trust.  

## Report issues

https://github.com/linuxbased79/FixItGarage/issues  
