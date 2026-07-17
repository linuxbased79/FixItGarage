# Changelog

## 0.2.20
- **Store-ready packaging**: `resources.arsc` stored uncompressed + page-aligned `.so` (fixes API 30+ install)
- Production signing via `FIG_KEYSTORE` env; `create-upload-keystore.sh`
- Play/F-Droid assets: screenshots, feature graphic, icon; full privacy policy URL
- `STORE.md` / updated `PLAY.md` / `F-DROID.md` submission guides

## 0.2.19
- **On-device OCR** (pure Rust [ocrs](https://github.com/robertknight/ocrs), no GMS): Capture → On-device OCR fills receipt fields
- OCR models (~12 MB) bundled in APK assets; first-run extract / optional download fallback
- **Tread CV**: coin-gauge computer vision estimates depth (mm / 32nds) from a penny-in-groove photo
- External OCR app / share-back path kept as fallback

## 0.2.18
- **Deep OCR pipeline**: Capture for OCR (MediaStore) → Send photo to Text Fairy / Lens → Share text back into FixItGarage
- Share target activity receives text/image from any OCR app (`ShareReceiveActivity`)
- Apply shared OCR / auto-fill on Receipt & Tires pages; OCR noise cleanup (Total/Oil/0↔O)
- Packaging compiles all Java bridges (Boot + Share)

## 0.2.17
- Receipt OCR assist: **Paste & fill** (clipboard + auto-parse in one tap) for service and tire receipts
- Metric receipts: km→mi, L→gal storage; €/£ amounts; EU DD/MM/YYYY dates
- OCR helper prefers installed apps (Text Fairy on F-Droid first) before Lens/web

## 0.2.16
- Release packaging: `scripts/release-apks.sh` builds both arches with BootReceiver
- F-Droid draft metadata (`metadata/org.fixitgarage.app.yml`) + updated listing text
- Play Console checklist (`PLAY.md`); README status brought current

## 0.2.15
- **Boot-resilient reminders**: `BootReceiver` re-registers date alarms after reboot / app update
- Alarm schedule saved to `fig_alarms.json` (app files dir) for the receiver
- Packaging script injects Java receiver into APK (`package-apk-with-boot.sh`)

## 0.2.14
- Accessibility: optional **OpenDyslexic** font (SIL OFL) for dyslexia-friendly reading
- Settings → Accessibility · reading: Default font / OpenDyslexic (saved on device)

## 0.2.13
- Asian language packs: **日本語**, **한국어**, **简体中文**
- System default detects ja / ko / zh locales
- Settings language picker lists the new packs

## 0.2.12
- Language packs: **System default** (follows phone OS) or override EN / ES / FR / DE
- Settings → Language; bottom nav, Settings, More, and key Home titles are translated
- Expand packs by adding keys in `i18n.rs`

## 0.2.11
- Tire positions / pattern / spare option are **per vehicle** (multi-car fix)
- Optional notes on service records (CSV export includes notes)
- Delete tire purchase from history

## 0.2.10
- Tire rotation: optional **Include spare** (off by default — most leave temporary spares out)
- 5-tire patterns when spare is on; spare shown on diagram; tread/mileage track spare

## 0.2.9
- Service quick templates: oil change, fuel fill, tire rotation, shop visit
- Parts log schedules smart reminders (air/cabin ~12 mo/15k; oil filter ~6 mo/5k)
- Operational costs include tire purchases; breakdown services vs tires

## 0.2.8
- Split navigation: **More** = tools/trackers hub; **Settings** (⚙) = preferences only
- Settings gear on Home header and bottom nav
- Appearance, units, feature focus, backup, cloud, support moved out of More

## 0.2.7
- Units preference: Imperial (mi, gal, MPG, tread 1/32\") or Metric (km, L, L/100km, tread mm)
- Settings toggle; saved on device; values convert for display
- Oil level choices switch between quarts and liters wording

## 0.2.6
- Smart reminders: multi-item notifications (all vehicles), 12h throttle
- AlarmManager date wake schedules so due checks re-open the app
- Home “Upcoming” list (90 days / 5k mi)
- Receipt OCR flow: clipboard paste + OCR helper + auto service title
- Tire receipt: clipboard + OCR helper buttons

## 0.2.5
- Tire rotation: live before/after top-down diagrams; tread + mileage follow corners
- Tire purchase receipt text parse (brand/model/size/cost/mileage)
- Service log: shop name + fuel cost fields
- Home alerts for brake service and aged wiper blades
- Coin-gauge guide for camera tread assist (penny ~1.6 mm)

## 0.2.2
- Home dashboard: MPG, month/year costs, reminder status
- Service history search/filter
- Fuel / MPG history screen with per-fill economy
- F-Droid metadata updates

## 0.2.1
- Due-reminder system notifications on launch
- Optional WebDAV / Nextcloud backup upload
- Receipt text auto-fill (paste OCR/email text)
- Edit selected vehicle details

## 0.2.0
- JSON backup / restore / share
- Share CSV
- Tread depth logging and 1.6 mm warning
- Receipt photo hook

## 0.1.x
- Initial multi-vehicle trackers, dark mode, oil level choices, wipers L/R, tires, parts, brakes, battery, notes, reminders

## 0.2.3
- Cloud backup buttons: Proton Drive (recommended), Google Drive, Dropbox
- Nextcloud/WebDAV section kept for direct upload

## 0.2.4
- OneDrive cloud backup share
- Brake/battery/wiper auto-reminders on save
- Mileage per tire; camera assist for tread
- Battery age home alert
