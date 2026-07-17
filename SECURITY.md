# Security notes — FixItGarage

Last reviewed with hardening in **0.2.24**.

## Design

- Local-first garage log; no FixItGarage account or analytics backend  
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

- On-device `state.json` still holds WebDAV password in app-private storage (needed for upload without re-entry every time). Protected by Android app sandbox; readable on a rooted device.  
- Shared backups / seller PDFs / CSV still contain **maintenance data** (by design).  
- VIN is sent to NHTSA when you run a recall check.  
- Sideload APKs may be debug-signed; use Play/F-Droid production signing for trust.  

## Report issues

https://github.com/linuxbased79/FixItGarage/issues  
