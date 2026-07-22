//! FixItGarage Slint UI — shared library for desktop binary and Android cdylib.

mod i18n;
mod ondevice_ocr;
mod platform;
mod receipt_parse;
mod recalls;
mod seller_packet;
mod state;
mod title_parse;
mod tread_cv;
mod units;
mod webdav;

use chrono::{TimeZone, Utc};
use i18n::{resolve_lang, t, Lang, LanguagePref};
use platform::{
    cancel_app_wake, capture_issue_photo_path, capture_receipt_for_ocr, capture_title_for_ocr,
    notify, notify_with_id, ocr_target, open_ocr_helper, open_ocr_helper_for_tire, open_url,
    pending_ocr_image_path, read_clipboard, schedule_app_wake, send_pending_image_to_ocr,
    set_ocr_target, share_file, share_text, share_text_to_cloud, system_locale,
    system_safe_area_dp, take_pending_ocr_text, write_alarm_schedule, PKG_DROPBOX,
    PKG_GOOGLE_DRIVE, PKG_ONEDRIVE, PKG_PROTON_DRIVE,
};
use receipt_parse::{parse_receipt_text, parse_tire_receipt_text};
use state::{reminder_status_line_units, AppState};
use std::sync::{Arc, Mutex};
use units::{
    display_to_gallons, display_to_miles, display_to_mm, format_economy, gallons_to_display,
    miles_to_display, mm_to_display, UnitSystem,
};

slint::include_modules!();

fn format_service_date(epoch_ms: i64) -> String {
    match Utc.timestamp_millis_opt(epoch_ms) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => String::new(),
    }
}

/// Localized oil-level choice labels (imperial quarts vs metric liters).
/// Storage always uses English imperial canonical via normalize_oil_level.
fn oil_level_labels(lang: Lang, units: UnitSystem) -> [String; 6] {
    let metric = matches!(units, UnitSystem::Metric);
    [
        t(lang, "oil.full"),
        if metric {
            t(lang, "oil.half_l")
        } else {
            t(lang, "oil.half_qt")
        },
        if metric {
            t(lang, "oil.1_l")
        } else {
            t(lang, "oil.1_qt")
        },
        if metric {
            t(lang, "oil.2_l")
        } else {
            t(lang, "oil.2_qt")
        },
        if metric {
            t(lang, "oil.3_l")
        } else {
            t(lang, "oil.3_qt")
        },
        t(lang, "oil.overfilled"),
    ]
}

fn oil_level_label(lang: Lang, units: UnitSystem, stored: &str) -> String {
    let n = state::normalize_oil_level(stored);
    let labels = oil_level_labels(lang, units);
    match n.as_str() {
        "Full" => labels[0].clone(),
        "½ quart low" => labels[1].clone(),
        "1 quart low" => labels[2].clone(),
        "2 quarts low" => labels[3].clone(),
        "3 quarts low" => labels[4].clone(),
        "Overfilled" => labels[5].clone(),
        other => other.to_string(),
    }
}

fn pattern_label_i18n(lang: Lang, pattern_id: &str) -> String {
    match pattern_id.trim().to_ascii_lowercase().as_str() {
        "forward_cross" | "forward" => t(lang, "tires.pat_fwd_full"),
        "rearward_cross" | "rearward" => t(lang, "tires.pat_rear_full"),
        "x_pattern" | "x" => t(lang, "tires.pat_x_full"),
        "side_to_side" | "side" => t(lang, "tires.pat_side_full"),
        other => other.to_string(),
    }
}

fn tire_preview_i18n(state: &AppState, lang: Lang) -> String {
    let cfg = state.selected_tire_config();
    let after = state.preview_after_layout();
    let pat = pattern_label_i18n(lang, &cfg.pattern);
    let mut s = format!(
        "{} {}: {} {} / {} {}",
        t(lang, "tires.after_word"),
        pat,
        after.fl,
        after.fr,
        after.rl,
        after.rr
    );
    if cfg.include_spare {
        s.push_str(&format!(" · SP {}", after.spare));
        s.push_str(&format!(" {}", t(lang, "tires.incl_spare_paren")));
    }
    s
}

fn last_oil_level_summary_i18n(state: &AppState, lang: Lang, u: UnitSystem) -> String {
    let Some(vid) = state.selected_vehicle_id else {
        return t(lang, "oil.no_vehicle");
    };
    state
        .oil_level_logs
        .iter()
        .rev()
        .find(|l| l.vehicle_id == vid)
        .map(|l| {
            let date = match Utc.timestamp_millis_opt(l.epoch_ms) {
                chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
                _ => "—".into(),
            };
            let mi = l
                .mileage
                .map(|m| format!(" · {}", units::format_distance(m, u)))
                .unwrap_or_default();
            let level = oil_level_label(lang, u, &l.level);
            format!("{} {}{} — {}", t(lang, "oil.last_check"), date, mi, level)
        })
        .unwrap_or_else(|| t(lang, "oil.none_logged"))
}

fn component_summary_i18n(state: &AppState, component_type: &str, lang: Lang) -> String {
    let Some(vid) = state.selected_vehicle_id else {
        return t(lang, "common.select_vehicle");
    };
    let entry = state
        .components
        .iter()
        .find(|c| c.vehicle_id == vid && c.component_type == component_type)
        .or_else(|| {
            if component_type == "WIPER_DRIVER" {
                state.components.iter().find(|c| {
                    c.vehicle_id == vid && c.component_type == "WIPER_FRONT"
                })
            } else {
                None
            }
        });
    match entry {
        Some(c) => {
            let date = c
                .installed_epoch_ms
                .map(format_service_date)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "—".into());
            let mi = c
                .installed_mileage
                .map(|m| units::format_distance(m, state.unit_system()))
                .unwrap_or_else(|| "—".into());
            let size = if c.notes.trim().is_empty() {
                t(lang, "comp.size_not_set")
            } else {
                c.notes.trim().to_string()
            };
            format!(
                "{} {date} · {mi}\n{} {size}",
                t(lang, "comp.installed"),
                t(lang, "comp.size_notes")
            )
        }
        None => {
            if component_type.starts_with("WIPER") {
                t(lang, "comp.no_entry_detail")
            } else {
                t(lang, "common.no_entry")
            }
        }
    }
}

fn part_summary_i18n(state: &AppState, part_type: &str, lang: Lang) -> String {
    let Some(vid) = state.selected_vehicle_id else {
        return t(lang, "common.select_vehicle");
    };
    match state
        .parts
        .iter()
        .find(|p| p.vehicle_id == vid && p.part_type == part_type)
    {
        Some(p) => {
            let mut s = format!("{} {}", p.brand, p.part_number);
            if !p.oil_viscosity.is_empty() {
                s.push_str(&format!(" · {}", p.oil_viscosity));
            }
            if !p.notes.is_empty() {
                s.push_str(&format!("\n{}", p.notes));
            }
            s
        }
        None => t(lang, "common.no_entry"),
    }
}

/// Push selected vehicle fields into the form (after save / load / select).
fn fill_form_from_selected(ui: &MainWindow, state: &AppState) {
    let Some(v) = state.selected_vehicle() else {
        return;
    };
    ui.set_form_name(v.name.clone().into());
    ui.set_form_make(v.make.clone().into());
    ui.set_form_model(v.model.clone().into());
    ui.set_form_vin(v.vin.clone().into());
    ui.set_form_year(v.year.map(|y| y.to_string()).unwrap_or_default().into());
    let u = state.unit_system();
    ui.set_form_mileage(miles_to_display(v.current_mileage, u).to_string().into());
}

/// If the user typed vehicle details but never tapped Save, persist them before
/// network work so a crash cannot wipe the form.
fn ensure_vehicle_from_form(ui: &MainWindow, state: &Arc<Mutex<AppState>>) {
    let mut s = state.lock().unwrap();
    let name = ui.get_form_name().to_string();
    let make = ui.get_form_make().to_string();
    let model = ui.get_form_model().to_string();
    let year = ui.get_form_year().parse().ok();
    let vin = ui.get_form_vin().to_string();
    let u = s.unit_system();
    let mileage = display_to_miles(ui.get_form_mileage().parse().unwrap_or(0), u);

    if s.selected_vehicle_id.is_some() {
        // Keep selected vehicle in sync with the form (VIN + details).
        let existing_name = s
            .selected_vehicle()
            .map(|v| v.name.clone())
            .unwrap_or_else(|| "My vehicle".into());
        let save_name = if name.trim().is_empty() {
            existing_name
        } else {
            name
        };
        s.update_selected_vehicle_details(
            save_name,
            make,
            model,
            year,
            Some(mileage).filter(|&m| m > 0),
            Some(vin),
        );
        return;
    }

    // No vehicle yet — create one if we have any identifying field.
    if name.trim().is_empty()
        && make.trim().is_empty()
        && model.trim().is_empty()
        && vin.trim().is_empty()
        && year.is_none()
    {
        return;
    }
    // add_vehicle auto-names from YMM/VIN when name is empty.
    s.add_vehicle(name, make, model, year, mileage, vin);
}

enum RecallOutcome {
    Vin {
        vin: String,
        result: Result<recalls::RecallCheckResult, String>,
    },
    Ymm {
        make: String,
        model: String,
        year: u16,
        result: Result<Vec<recalls::RecallItem>, String>,
    },
}

fn apply_recall_outcome(ui: &MainWindow, state: &Arc<Mutex<AppState>>, outcome: RecallOutcome) {
    match outcome {
        RecallOutcome::Ymm {
            make,
            model,
            year,
            result,
        } => match result {
            Ok(list) => {
                let rows: Vec<RecallRow> = list
                    .iter()
                    .map(|r| RecallRow {
                        campaign: r.campaign.clone().into(),
                        component: r.component.clone().into(),
                        summary: truncate_ui(&r.summary, 280).into(),
                        detail: format!("{} · {}", r.report_date, r.manufacturer).into(),
                    })
                    .collect();
                let n = rows.len();
                ui.set_recalls(rows.as_slice().into());
                ui.set_recall_status(
                    format!(
                        "NHTSA: {n} recall campaign(s) for {year} {make} {model}. Free repairs at dealer when open for your VIN."
                    )
                    .into(),
                );
                ui.set_status_message(format!("Found {n} recall(s).").into());
            }
            Err(e) => {
                ui.set_recall_status(format!("Recall check failed: {e}").into());
                ui.set_status_message(format!("Recalls: {e}").into());
            }
        },
        RecallOutcome::Vin { vin, result } => match result {
            Ok(result) => {
                let vin_norm = recalls::normalize_vin(&vin).unwrap_or(vin);
                {
                    let mut s = state.lock().unwrap();
                    // If still no vehicle, create one from decode.
                    if s.selected_vehicle_id.is_none() {
                        let name = format!(
                            "{} {} {}",
                            result.decoded.year.map(|y| y.to_string()).unwrap_or_default(),
                            result.decoded.make,
                            result.decoded.model
                        )
                        .trim()
                        .to_string();
                        s.add_vehicle(
                            if name.is_empty() {
                                "My vehicle".into()
                            } else {
                                name
                            },
                            result.decoded.make.clone(),
                            result.decoded.model.clone(),
                            result.decoded.year,
                            0,
                            vin_norm.clone(),
                        );
                    } else {
                        s.apply_vin_decode_to_selected(
                            vin_norm.clone(),
                            Some(result.decoded.make.clone()),
                            Some(result.decoded.model.clone()),
                            result.decoded.year,
                        );
                    }
                }
                ui.set_form_vin(vin_norm.into());
                if !result.decoded.make.is_empty() {
                    ui.set_form_make(result.decoded.make.clone().into());
                }
                if !result.decoded.model.is_empty() {
                    ui.set_form_model(result.decoded.model.clone().into());
                }
                if let Some(y) = result.decoded.year {
                    ui.set_form_year(y.to_string().into());
                }
                let rows: Vec<RecallRow> = result
                    .recalls
                    .iter()
                    .map(|r| RecallRow {
                        campaign: r.campaign.clone().into(),
                        component: r.component.clone().into(),
                        summary: truncate_ui(&r.summary, 280).into(),
                        detail: format!("{} · {}", r.report_date, r.manufacturer).into(),
                    })
                    .collect();
                let n = rows.len();
                ui.set_recalls(rows.as_slice().into());
                let status = if n == 0 {
                    format!(
                        "No NHTSA campaigns listed for {} {} {}. Still verify open status on NHTSA VIN page.",
                        result.decoded.year.unwrap_or(0),
                        result.decoded.make,
                        result.decoded.model
                    )
                } else {
                    format!(
                        "{n} campaign(s) for {} {} {}. Use “Open NHTSA VIN page” to see if this VIN still needs repair.",
                        result.decoded.year.unwrap_or(0),
                        result.decoded.make,
                        result.decoded.model
                    )
                };
                ui.set_recall_status(status.into());
                ui.set_status_message(format!("Recalls: {n} found. Vehicle saved.").into());
                refresh_ui(ui, &state.lock().unwrap());
            }
            Err(e) => {
                ui.set_recall_status(format!("Recall check failed: {e}").into());
                ui.set_status_message(format!("Recalls: {e}").into());
                // Still refresh so a just-saved vehicle appears in the list.
                refresh_ui(ui, &state.lock().unwrap());
            }
        },
    }
}

/// Wire Slint properties/callbacks to AppState and show the window.
pub fn run_app() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    // Resolve Android files dir only after the native activity/JNI context exists.
    let data_dir = crate::platform::app_data_dir();
    eprintln!("FixItGarage: using data dir {}", data_dir.display());
    let state = Arc::new(Mutex::new(AppState::load()));

    // Lift chrome above system bars (status / 3-button or gesture navigation).
    // Without this, the bottom Settings tab sits under △ ○ □ and cannot be tapped.
    // Slint length properties take Coord (logical px / dp).
    {
        let (top_dp, bottom_dp) = system_safe_area_dp();
        ui.set_safe_area_top(top_dp);
        // Extra few dp so the last tab never sits in the system gesture dead-zone.
        ui.set_safe_area_bottom(bottom_dp + 8.0);
    }

    refresh_ui(&ui, &state.lock().unwrap());
    // Due notifications (all vehicles) + schedule future date alarms
    {
        let mut s = state.lock().unwrap();
        fire_due_notifications(&mut s);
        reschedule_reminder_alarms(&s);
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_finish_wizard(move |mode| {
            let mut s = state.lock().unwrap();
            s.wizard_done = true;
            s.user_mode = mode.to_string();
            s.save();
            if let Some(ui) = ui_weak.upgrade() {
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_navigate(move |page| {
            let s = state.lock().unwrap();
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_page(page);
                refresh_ui(&ui, &s);
                // Receipt or tire pages: pull shared OCR text / image if any
                if page == 13 || page == 3 {
                    try_consume_pending_ocr(&ui, &s, page == 3);
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_select_vehicle(move |id| {
            let mut s = state.lock().unwrap();
            s.select_vehicle(id as u64);
            if let Some(ui) = ui_weak.upgrade() {
                if let Some(v) = s.selected_vehicle() {
                    ui.set_form_name(v.name.clone().into());
                    ui.set_form_make(v.make.clone().into());
                    ui.set_form_model(v.model.clone().into());
                    ui.set_form_vin(v.vin.clone().into());
                    ui.set_form_year(
                        v.year.map(|y| y.to_string()).unwrap_or_default().into(),
                    );
                    let u = s.unit_system();
                    ui.set_form_mileage(
                        miles_to_display(v.current_mileage, u).to_string().into(),
                    );
                }
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_add_vehicle(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let name = ui.get_form_name().to_string();
                let make = ui.get_form_make().to_string();
                let model = ui.get_form_model().to_string();
                let year = ui.get_form_year().parse().ok();
                let vin = ui.get_form_vin().to_string();
                let u = s.unit_system();
                let mileage = display_to_miles(
                    ui.get_form_mileage().parse().unwrap_or(0),
                    u,
                );
                // If the form still matches the selected vehicle (same name or same VIN),
                // treat Save as an update so users don't create accidental duplicates.
                // Otherwise add a new car (second vehicle, OCR of a different title, etc.).
                let same_as_selected = s.selected_vehicle().map(|v| {
                    let same_vin = !vin.trim().is_empty()
                        && !v.vin.is_empty()
                        && v.vin.eq_ignore_ascii_case(vin.trim());
                    let same_name = !name.trim().is_empty()
                        && v.name.eq_ignore_ascii_case(name.trim());
                    same_vin || same_name
                }).unwrap_or(false);
                if same_as_selected {
                    let existing_name = s
                        .selected_vehicle()
                        .map(|v| v.name.clone())
                        .unwrap_or_else(|| "My vehicle".into());
                    let save_name = if name.trim().is_empty() {
                        existing_name
                    } else {
                        name.clone()
                    };
                    s.update_selected_vehicle_details(
                        save_name,
                        make.clone(),
                        model.clone(),
                        year,
                        Some(mileage).filter(|&m| m > 0),
                        Some(vin.clone()),
                    );
                    let ok = s.save();
                    fill_form_from_selected(&ui, &s);
                    refresh_ui(&ui, &s);
                    ui.set_status_message(
                        if ok {
                            format!(
                                "Vehicle updated and saved ({} on this device).",
                                s.vehicles.len()
                            )
                            .into()
                        } else {
                            "WARNING: update may not have been written. Tap Save again.".into()
                        },
                    );
                    return;
                }
                let before = s.vehicles.len();
                s.add_vehicle(name, make, model, year, mileage, vin);
                let after = s.vehicles.len();
                // Force another durable write (add_vehicle already saves; verify path works).
                let ok = s.save();
                // Keep the saved vehicle visible in the form (do NOT clear — that looked like
                // "save failed" and left empty fields after restart until user re-tapped).
                fill_form_from_selected(&ui, &s);
                if after <= before {
                    ui.set_status_message(
                        "Could not add vehicle — enter a Name, VIN, or Make first.".into(),
                    );
                } else if ok {
                    ui.set_status_message(
                        format!(
                            "Vehicle saved ({} total). Still here after restart.",
                            after
                        )
                        .into(),
                    );
                } else {
                    ui.set_status_message(
                        "WARNING: vehicle may not have been written to storage. Try Save again."
                            .into(),
                    );
                }
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_update_selected_mileage(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let Some(id) = s.selected_vehicle_id else {
                    ui.set_status_message("Select a vehicle first.".into());
                    return;
                };
                let u = s.unit_system();
                let mileage = display_to_miles(
                    ui.get_form_mileage().parse().unwrap_or(0),
                    u,
                );
                s.update_vehicle_mileage(id, mileage);
                ui.set_status_message(format!(
                    "Odometer updated to {}.",
                    units::format_distance(mileage, u)
                ).into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_update_selected_vehicle(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let miles = ui
                    .get_form_mileage()
                    .parse()
                    .ok()
                    .map(|v| display_to_miles(v, u));
                s.update_selected_vehicle_details(
                    ui.get_form_name().to_string(),
                    ui.get_form_make().to_string(),
                    ui.get_form_model().to_string(),
                    ui.get_form_year().parse().ok(),
                    miles,
                    Some(ui.get_form_vin().to_string()),
                );
                ui.set_status_message("Vehicle details updated.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_check_recalls(move || {
            if let Some(ui) = ui_weak.upgrade() {
                // Persist form to disk first so a crash/network kill cannot wipe typed data.
                ensure_vehicle_from_form(&ui, &state);

                let mut vin = ui.get_form_vin().to_string();
                if vin.trim().is_empty() {
                    if let Some(v) = state.lock().unwrap().selected_vehicle() {
                        vin = v.vin.clone();
                    }
                }
                let make = ui.get_form_make().to_string();
                let model = ui.get_form_model().to_string();
                let year: u16 = ui.get_form_year().parse().unwrap_or(0);

                ui.set_recalls(Vec::<RecallRow>::new().as_slice().into());
                if vin.trim().is_empty() {
                    ui.set_recall_status("Checking NHTSA by make/model/year…".into());
                } else {
                    ui.set_recall_status("Contacting NHTSA (VIN decode + recalls)…".into());
                }
                ui.set_status_message("Checking recalls…".into());

                let ui_weak = ui_weak.clone();
                let state = state.clone();
                // Network must not run on the UI thread (Android force-closes / ANR).
                std::thread::spawn(move || {
                    let outcome = if vin.trim().is_empty() {
                        RecallOutcome::Ymm {
                            make: make.clone(),
                            model: model.clone(),
                            year,
                            result: recalls::check_recalls_ymm(&make, &model, year),
                        }
                    } else {
                        RecallOutcome::Vin {
                            vin: vin.clone(),
                            result: recalls::check_recalls_for_vin(&vin),
                        }
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            apply_recall_outcome(&ui, &state, outcome);
                        }
                    });
                });
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_nhtsa_vin_page(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut vin = ui.get_form_vin().to_string();
                if vin.trim().is_empty() {
                    if let Some(v) = state.lock().unwrap().selected_vehicle() {
                        vin = v.vin.clone();
                    }
                }
                match recalls::normalize_vin(&vin) {
                    Ok(v) => {
                        open_url(&recalls::nhtsa_vin_web_url(&v));
                        ui.set_status_message("Opened NHTSA recalls page for this VIN.".into());
                    }
                    Err(e) => ui.set_status_message(format!("VIN: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_scan_vehicle_title(move || {
            if let Some(ui) = ui_weak.upgrade() {
                // MediaStore EXTRA_OUTPUT capture — same path as receipt OCR (writes a real photo).
                let path = capture_title_for_ocr();
                ui.set_title_photo_path(path.into());
                ui.set_status_message(
                    "Camera opened — photograph the full title (or VIN plate), return here, wait a second, then tap Read photo (OCR)."
                        .into(),
                );
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_ocr_vehicle_title(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Reading title photo (on-device OCR)…".into());
                let ui_weak = ui_weak.clone();
                let state = state.clone();
                let hinted = ui.get_title_photo_path().to_string();
                std::thread::spawn(move || {
                    // Camera may still be writing MediaStore — retry briefly.
                    let mut path: Option<String> = None;
                    for attempt in 0..8 {
                        if let Some(img) = pending_ocr_image_path() {
                            path = Some(img);
                            break;
                        }
                        if !hinted.trim().is_empty()
                            && !hinted.starts_with("content:")
                            && std::path::Path::new(&hinted).exists()
                        {
                            if let Ok(meta) = std::fs::metadata(&hinted) {
                                if meta.len() > 32 {
                                    path = Some(hinted.clone());
                                    break;
                                }
                            }
                        }
                        if attempt + 1 < 8 {
                            std::thread::sleep(std::time::Duration::from_millis(400));
                        }
                    }
                    let ocr_result = match path {
                        Some(ref p) => ondevice_ocr::ocr_image_path(p).map(|t| (p.clone(), t)),
                        None => Err(
                            "No title photo yet — tap Photo title / VIN, take the picture, return, then Read photo."
                                .into(),
                        ),
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            match ocr_result {
                                Ok((p, text)) => {
                                    ui.set_title_photo_path(p.into());
                                    apply_title_ocr_text(&ui, &state, &text);
                                }
                                Err(e) => ui.set_status_message(
                                    format!(
                                        "Title OCR: {e}"
                                    )
                                    .into(),
                                ),
                            }
                        }
                    });
                });
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_selected_vehicle(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let Some(id) = s.selected_vehicle_id else {
                    ui.set_status_message("Select a vehicle first.".into());
                    return;
                };
                s.delete_vehicle(id);
                ui.set_status_message("Vehicle and related data deleted.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_service(move |id| {
            let mut s = state.lock().unwrap();
            s.delete_service(id as u64);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Service deleted.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_note(move |id| {
            let mut s = state.lock().unwrap();
            s.delete_note(id as u64);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Note deleted.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_tread(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let parse = |v: slint::SharedString| {
                    let t = v.to_string();
                    if t.trim().is_empty() {
                        None
                    } else {
                        t.parse::<f64>().ok().map(|x| display_to_mm(x, u))
                    }
                };
                s.set_tread_depths(
                    parse(ui.get_tread_fl()),
                    parse(ui.get_tread_fr()),
                    parse(ui.get_tread_rl()),
                    parse(ui.get_tread_rr()),
                    parse(ui.get_tread_spare()),
                );
                ui.set_status_message("Tread depths saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_capture_tread_photo(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let path = capture_issue_photo_path();
                ui.set_tread_photo_path(path.into());
                ui.set_status_message(
                    "Camera opened — use coin gauge (penny ~1.6 mm), enter mm, then Save tread depths."
                        .into(),
                );
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_parse_tire_receipt(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let paste = ui.get_tire_rcp_paste().to_string();
                if paste.trim().is_empty() {
                    ui.set_status_message("Paste tire receipt text first.".into());
                    return;
                }
                let p = parse_tire_receipt_text(&paste);
                let u = state.lock().unwrap().unit_system();
                let mut filled = 0u32;
                if let Some(b) = p.brand {
                    ui.set_tire_brand(b.into());
                    filled += 1;
                }
                if let Some(m) = p.model {
                    ui.set_tire_model(m.into());
                    filled += 1;
                }
                if let Some(sz) = p.size {
                    ui.set_tire_size(sz.into());
                    filled += 1;
                }
                if let Some(c) = p.cost {
                    ui.set_tire_cost(format!("{c:.2}").into());
                    filled += 1;
                }
                if let Some(mi) = p.mileage {
                    ui.set_tire_buy_mileage(miles_to_display(mi, u).to_string().into());
                    filled += 1;
                }
                if let Some(n) = p.notes {
                    if ui.get_tire_buy_notes().is_empty() {
                        ui.set_tire_buy_notes(n.into());
                    }
                }
                ui.set_status_message(
                    format!("Tire receipt parsed ({filled} fields). Review and Save.").into(),
                );
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_tire_miles(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let parse = |v: slint::SharedString| {
                    let t = v.to_string();
                    if t.trim().is_empty() {
                        None
                    } else {
                        t.parse::<u32>().ok().map(|x| display_to_miles(x, u))
                    }
                };
                s.set_tire_corner_miles(
                    parse(ui.get_mi_fl()),
                    parse(ui.get_mi_fr()),
                    parse(ui.get_mi_rl()),
                    parse(ui.get_mi_rr()),
                    parse(ui.get_mi_spare()),
                );
                ui.set_status_message("Distance per tire saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_fill_service_template(move |kind| {
            if let Some(ui) = ui_weak.upgrade() {
                let s = state.lock().unwrap();
                let u = s.unit_system();
                let lang = resolve_lang(s.language_pref(), &system_locale());
                let odo = s
                    .selected_vehicle()
                    .map(|v| miles_to_display(v.current_mileage, u))
                    .unwrap_or(0);
                let k = kind.to_string().to_ascii_lowercase();
                match k.as_str() {
                    "oil" => {
                        ui.set_svc_title(t(lang, "service.oil_change").into());
                        ui.set_svc_source("DIY".into());
                        ui.set_svc_mileage(if odo > 0 { odo.to_string().into() } else { "".into() });
                        ui.set_svc_gallons("".into());
                        ui.set_svc_fuel("".into());
                        ui.set_svc_shop("".into());
                        ui.set_status_message(t(lang, "template.oil_status").into());
                    }
                    "fuel" => {
                        ui.set_svc_title(t(lang, "service.fuel_fillup_title").into());
                        ui.set_svc_source("DIY".into());
                        ui.set_svc_mileage(if odo > 0 { odo.to_string().into() } else { "".into() });
                        ui.set_svc_cost("".into());
                        ui.set_svc_labor("".into());
                        ui.set_svc_shop("".into());
                        ui.set_status_message(
                            t(lang, "template.fuel_status")
                                .replace("{fuel}", u.fuel_label())
                                .into(),
                        );
                    }
                    "rotation" => {
                        ui.set_svc_title(t(lang, "service.tire_rotation_title").into());
                        ui.set_svc_source("DIY".into());
                        ui.set_svc_mileage(if odo > 0 { odo.to_string().into() } else { "".into() });
                        ui.set_svc_gallons("".into());
                        ui.set_svc_fuel("".into());
                        ui.set_status_message(t(lang, "template.rotation_status").into());
                    }
                    "shop" => {
                        ui.set_svc_title(t(lang, "service.shop_visit_title").into());
                        ui.set_svc_source("SHOP".into());
                        ui.set_svc_mileage(if odo > 0 { odo.to_string().into() } else { "".into() });
                        ui.set_svc_gallons("".into());
                        ui.set_status_message(t(lang, "template.shop_status").into());
                    }
                    _ => {
                        ui.set_status_message("Unknown template.".into());
                    }
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_add_service(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                if s.vehicles.is_empty() {
                    ui.set_status_message("Add a vehicle first.".into());
                    return;
                }
                let u = s.unit_system();
                let title = ui.get_svc_title().to_string();
                let mileage = display_to_miles(
                    ui.get_svc_mileage().parse().unwrap_or(0),
                    u,
                );
                let parts = ui.get_svc_cost().parse().unwrap_or(0.0);
                let labor = ui.get_svc_labor().parse().unwrap_or(0.0);
                let gallons = {
                    let g = ui.get_svc_gallons().to_string();
                    if g.trim().is_empty() {
                        None
                    } else {
                        g.parse::<f64>().ok().map(|v| display_to_gallons(v, u))
                    }
                };
                let fuel_cost = {
                    let f = ui.get_svc_fuel().to_string();
                    if f.trim().is_empty() {
                        None
                    } else {
                        f.parse().ok()
                    }
                };
                let shop = ui.get_svc_shop().to_string();
                let notes = ui.get_svc_notes().to_string();
                let source = ui.get_svc_source().to_string();
                s.add_service_full(
                    title,
                    mileage,
                    &source,
                    parts,
                    labor,
                    gallons,
                    fuel_cost,
                    chrono::Utc::now().timestamp_millis(),
                    shop,
                    notes,
                );
                ui.set_svc_title("".into());
                ui.set_svc_mileage("".into());
                ui.set_svc_cost("".into());
                ui.set_svc_labor("".into());
                ui.set_svc_gallons("".into());
                ui.set_svc_fuel("".into());
                ui.set_svc_shop("".into());
                ui.set_svc_notes("".into());
                ui.set_status_message("Service saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_source(move |src| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_svc_source(src);
                refresh_ui(&ui, &state.lock().unwrap());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_pattern(move |pattern| {
            let mut s = state.lock().unwrap();
            s.set_tire_pattern(pattern.to_string());
            if let Some(ui) = ui_weak.upgrade() {
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_include_spare(move |include| {
            let mut s = state.lock().unwrap();
            s.set_include_spare(include);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message(
                    if include {
                        "Spare included in rotation (full-size spare only)."
                    } else {
                        "4-tire rotation — spare stays put (default)."
                    }
                    .into(),
                );
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_spare_label(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                s.set_spare_label(ui.get_tire_spare().to_string());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_apply_rotation(move || {
            let mut s = state.lock().unwrap();
            let with_spare = s.selected_tire_config().include_spare;
            s.apply_tire_rotation();
            s.save();
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message(
                    if with_spare {
                        "5-tire rotation applied (spare included)."
                    } else {
                        "4-tire rotation applied."
                    }
                    .into(),
                );
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_user_mode(move |mode| {
            let mut s = state.lock().unwrap();
            s.user_mode = mode.to_string();
            s.save();
            if let Some(ui) = ui_weak.upgrade() {
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_units(move |units| {
            let mut s = state.lock().unwrap();
            s.set_units(&units);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message(format!(
                    "Units: {} (saved). Values convert for display; data stored as miles/gallons/mm.",
                    s.unit_system().as_str()
                ).into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_language(move |lang| {
            let mut s = state.lock().unwrap();
            s.set_language(&lang);
            if let Some(ui) = ui_weak.upgrade() {
                let pref = s.language_pref();
                let resolved = resolve_lang(pref, &system_locale());
                let msg = if pref == LanguagePref::System {
                    t(resolved, "status.language_system")
                } else {
                    format!(
                        "{} ({})",
                        t(resolved, "status.language_set"),
                        pref.label()
                    )
                };
                ui.set_status_message(msg.into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_dyslexia_font(move |enabled| {
            let mut s = state.lock().unwrap();
            s.set_dyslexia_font(enabled);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message(
                    if enabled {
                        "OpenDyslexic font on — easier reading for many dyslexic folks."
                    } else {
                        "Back to the default system font."
                    }
                    .into(),
                );
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_dark_mode(move |mode| {
            let mut s = state.lock().unwrap();
            let mode = mode.to_string();
            s.dark_mode = if mode.eq_ignore_ascii_case("DARK") {
                "DARK".into()
            } else {
                "LIGHT".into()
            };
            s.save();
            if let Some(ui) = ui_weak.upgrade() {
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_part(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let mi = ui
                    .get_part_mileage()
                    .parse()
                    .ok()
                    .map(|v| display_to_miles(v, u));
                let ptype = ui.get_part_type().to_string();
                s.upsert_part(
                    ptype.clone(),
                    ui.get_part_brand().to_string(),
                    ui.get_part_number().to_string(),
                    ui.get_part_oil().to_string(),
                    ui.get_part_notes().to_string(),
                    mi,
                );
                let msg = if matches!(
                    ptype.as_str(),
                    "ENGINE_AIR_FILTER" | "CABIN_FILTER" | "OIL_FILTER"
                ) {
                    "Part saved — replacement reminder scheduled."
                } else {
                    "Part saved."
                };
                ui.set_status_message(msg.into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_component(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let mi = ui
                    .get_comp_mileage()
                    .parse()
                    .ok()
                    .map(|v| display_to_miles(v, u));
                s.upsert_component(
                    ui.get_comp_type().to_string(),
                    ui.get_comp_notes().to_string(),
                    mi,
                    &ui.get_comp_date().to_string(),
                );
                ui.set_status_message("Component saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_add_note(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                s.add_note(ui.get_note_title().to_string(), ui.get_note_body().to_string());
                ui.set_note_title("".into());
                ui.set_note_body("".into());
                ui.set_status_message("Note saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_add_reminder(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let due_mi = ui
                    .get_rem_mileage()
                    .parse()
                    .ok()
                    .map(|v| display_to_miles(v, u));
                s.add_reminder(
                    ui.get_rem_title().to_string(),
                    &ui.get_rem_date().to_string(),
                    due_mi,
                );
                reschedule_reminder_alarms(&s);
                ui.set_rem_title("".into());
                ui.set_rem_date("".into());
                ui.set_rem_mileage("".into());
                ui.set_status_message("Reminder saved (date alarms scheduled on Android).".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_complete_reminder(move |id| {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let oil = ui.get_oil_level_choice().to_string();
                cancel_app_wake(50000 + (id % 10000));
                s.complete_reminder(id as u64, Some(&oil));
                reschedule_reminder_alarms(&s);
                ui.set_status_message(format!("Logged oil level: {oil}").into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_oil_level_choice(move |choice| {
            if let Some(ui) = ui_weak.upgrade() {
                let choice_s = choice.to_string();
                ui.set_oil_level_choice(choice_s.clone().into());
                let s = state.lock().unwrap();
                let lang = resolve_lang(s.language_pref(), &system_locale());
                let u = s.unit_system();
                ui.set_oil_level_display(oil_level_label(lang, u, &choice_s).into());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_capture_photo(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let path = capture_issue_photo_path();
                ui.set_photo_path(path.clone().into());
                ui.set_status_message("Camera opened (or path ready). Add caption and Save.".into());
                let _ = state;
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_add_photo(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                s.add_issue_photo(
                    ui.get_photo_caption().to_string(),
                    ui.get_photo_notes().to_string(),
                    ui.get_photo_path().to_string(),
                );
                ui.set_photo_caption("".into());
                ui.set_photo_notes("".into());
                ui.set_photo_path("".into());
                ui.set_status_message("Issue photo logged.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_receipt(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                if s.selected_vehicle_id.is_none() {
                    ui.set_status_message("Select a vehicle first.".into());
                    return;
                }
                let u = s.unit_system();
                let gallons = {
                    let g = ui.get_rcp_gallons().to_string();
                    if g.trim().is_empty() {
                        None
                    } else {
                        g.parse::<f64>().ok().map(|v| display_to_gallons(v, u))
                    }
                };
                let fuel = {
                    let f = ui.get_rcp_fuel().to_string();
                    if f.trim().is_empty() {
                        None
                    } else {
                        f.parse().ok()
                    }
                };
                s.add_receipt(
                    ui.get_rcp_title().to_string(),
                    &ui.get_rcp_date().to_string(),
                    display_to_miles(ui.get_rcp_mileage().parse().unwrap_or(0), u),
                    gallons,
                    ui.get_rcp_parts().parse().unwrap_or(0.0),
                    ui.get_rcp_labor().parse().unwrap_or(0.0),
                    fuel,
                    ui.get_rcp_shop().to_string(),
                    "SHOP",
                    String::new(),
                );
                ui.set_rcp_title("".into());
                ui.set_rcp_date("".into());
                ui.set_rcp_mileage("".into());
                ui.set_rcp_gallons("".into());
                ui.set_rcp_parts("".into());
                ui.set_rcp_labor("".into());
                ui.set_rcp_fuel("".into());
                ui.set_rcp_shop("".into());
                ui.set_status_message("Receipt saved to maintenance history.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_tire_purchase(move |id| {
            let mut s = state.lock().unwrap();
            s.delete_tire_purchase(id as u64);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Tire purchase deleted.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_tire_purchase(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let u = s.unit_system();
                let mi = ui
                    .get_tire_buy_mileage()
                    .parse()
                    .ok()
                    .map(|v| display_to_miles(v, u));
                s.add_tire_purchase(
                    ui.get_tire_brand().to_string(),
                    ui.get_tire_model().to_string(),
                    ui.get_tire_size().to_string(),
                    ui.get_tire_cost().parse().unwrap_or(0.0),
                    mi,
                    ui.get_tire_buy_notes().to_string(),
                );
                ui.set_tire_brand("".into());
                ui.set_tire_model("".into());
                ui.set_tire_size("".into());
                ui.set_tire_cost("".into());
                ui.set_tire_buy_mileage("".into());
                ui.set_tire_buy_notes("".into());
                ui.set_status_message("Tire purchase saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_export_csv(move || {
            let s = state.lock().unwrap();
            let csv = s.export_csv();
            let path = AppState::data_path().with_file_name("export.csv");
            let _ = std::fs::write(&path, &csv);
            if let Some(ui) = ui_weak.upgrade() {
                let preview: String = csv.chars().take(400).collect();
                ui.set_csv_preview(preview.into());
                ui.set_status_message(format!("Exported to {}", path.display()).into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_share_csv(move || {
            let s = state.lock().unwrap();
            let csv = s.export_csv();
            share_text("FixItGarage CSV export", &csv);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Share sheet opened (or file saved).".into());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_backup_json(move || {
            let s = state.lock().unwrap();
            match s.write_backup_file() {
                Ok(path) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_backup_path(path.display().to_string().into());
                        ui.set_status_message(format!("Backup written: {}", path.display()).into());
                        refresh_ui(&ui, &s);
                    }
                }
                Err(e) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_message(format!("Backup failed: {e}").into());
                    }
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_restore_json(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let path = ui.get_backup_path().to_string();
                match AppState::restore_from_file(&path) {
                    Ok(new_state) => {
                        *state.lock().unwrap() = new_state;
                        ui.set_status_message("Backup restored.".into());
                        refresh_ui(&ui, &state.lock().unwrap());
                    }
                    Err(e) => {
                        ui.set_status_message(format!("Restore failed: {e}").into());
                    }
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_share_backup(move || {
            let s = state.lock().unwrap();
            match s.write_backup_file() {
                Ok(path) => {
                    if let Ok(json) = std::fs::read_to_string(&path) {
                        share_text("FixItGarage backup", &json);
                    }
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_backup_path(path.display().to_string().into());
                        ui.set_status_message("Backup shared / saved.".into());
                    }
                }
                Err(e) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_message(format!("Share backup failed: {e}").into());
                    }
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_share_backup_to(move |target| {
            let s = state.lock().unwrap();
            let target = target.to_string();
            match s.write_backup_file() {
                Ok(path) => {
                    let json = std::fs::read_to_string(&path).unwrap_or_default();
                    let subject = "FixItGarage backup";
                    let (pkg, label) = match target.as_str() {
                        "proton" => (PKG_PROTON_DRIVE, "Proton Drive"),
                        "gdrive" => (PKG_GOOGLE_DRIVE, "Google Drive"),
                        "dropbox" => (PKG_DROPBOX, "Dropbox"),
                        "onedrive" => (PKG_ONEDRIVE, "OneDrive"),
                        _ => {
                            share_text(subject, &json);
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_backup_path(path.display().to_string().into());
                                ui.set_status_message("Share sheet opened.".into());
                            }
                            return;
                        }
                    };
                    share_text_to_cloud(subject, &json, pkg, label);
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_backup_path(path.display().to_string().into());
                        ui.set_status_message(
                            format!(
                                "Opening {label}… Install the app if prompted, then save the backup file."
                            )
                            .into(),
                        );
                    }
                }
                Err(e) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_message(format!("Backup failed: {e}").into());
                    }
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_capture_receipt_photo(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let path = capture_issue_photo_path();
                ui.set_rcp_photo_path(path.into());
                ui.set_status_message(
                    "Camera opened — after photo, use Send photo to OCR or Paste & fill.".into(),
                );
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_capture_receipt_for_ocr(move || {
            if let Some(ui) = ui_weak.upgrade() {
                set_ocr_target("receipt");
                let path = capture_receipt_for_ocr();
                ui.set_rcp_photo_path(path.into());
                // Best-effort: if a previous share image exists, show it
                if let Some(img) = pending_ocr_image_path() {
                    ui.set_rcp_photo_path(img.into());
                }
                ui.set_status_message(
                    "Camera for OCR — take the photo, return here, then Send photo to OCR.".into(),
                );
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_send_receipt_to_ocr(move || {
            if let Some(ui) = ui_weak.upgrade() {
                set_ocr_target("receipt");
                // Finalize camera URI → local file if needed
                if let Some(img) = pending_ocr_image_path() {
                    ui.set_rcp_photo_path(img.into());
                }
                match send_pending_image_to_ocr() {
                    Ok(()) => ui.set_status_message(
                        "Opened OCR app with photo. Share text back to FixItGarage or Copy, then Apply shared OCR / Paste & fill.".into(),
                    ),
                    Err(e) => ui.set_status_message(format!("Send to OCR: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_ocr_receipt_on_device(move || {
            if let Some(ui) = ui_weak.upgrade() {
                set_ocr_target("receipt");
                ui.set_status_message("Running on-device OCR… (first run may load models)".into());
                let path = {
                    let p = ui.get_rcp_photo_path().to_string();
                    if !p.is_empty() && std::path::Path::new(&p).is_file() {
                        p
                    } else if let Some(img) = pending_ocr_image_path() {
                        ui.set_rcp_photo_path(img.clone().into());
                        img
                    } else {
                        ui.set_status_message(
                            "Capture or share a receipt photo first, then run On-device OCR.".into(),
                        );
                        return;
                    }
                };
                match ondevice_ocr::ocr_image_path(&path) {
                    Ok(text) => {
                        let s = state.lock().unwrap();
                        apply_receipt_ocr_text(&ui, &s, &text);
                    }
                    Err(e) => ui.set_status_message(format!("On-device OCR: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_measure_tread_cv(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Analyzing tread photo (coin gauge CV)…".into());
                let path = {
                    let p = ui.get_tread_photo_path().to_string();
                    if !p.is_empty() && std::path::Path::new(&p).is_file() {
                        p
                    } else if let Some(img) = pending_ocr_image_path() {
                        // Shared image can be used for tread too
                        ui.set_tread_photo_path(img.clone().into());
                        img
                    } else {
                        let p2 = capture_issue_photo_path();
                        ui.set_tread_photo_path(p2.into());
                        ui.set_status_message(
                            "Camera opened — place a US penny in the tread groove, take the photo, then tap Measure tread (CV) again.".into(),
                        );
                        return;
                    }
                };
                match tread_cv::estimate_tread_from_image(&path) {
                    Ok(est) => {
                        let u = state.lock().unwrap().unit_system();
                        let shown = mm_to_display(est.depth_mm, u);
                        let s = format!("{shown:.1}");
                        ui.set_tread_fl(s.clone().into());
                        ui.set_tread_fr(s.clone().into());
                        ui.set_tread_rl(s.clone().into());
                        ui.set_tread_rr(s.clone().into());
                        ui.set_status_message(
                            format!(
                                "Tread CV ≈ {shown:.1} {} (conf {:.0}%). {} Review each corner, then Save.",
                                if matches!(u, units::UnitSystem::Imperial) {
                                    "/32\""
                                } else {
                                    "mm"
                                },
                                est.confidence * 100.0,
                                est.note
                            )
                            .into(),
                        );
                    }
                    Err(e) => ui.set_status_message(format!("Tread CV: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_apply_pending_ocr(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let s = state.lock().unwrap();
                let tire = ocr_target() == "tire" || ui.get_page() == 3;
                if try_consume_pending_ocr(&ui, &s, tire) {
                    // status set inside helper
                } else {
                    // Soft fallback: clipboard
                    if let Some(text) = read_clipboard() {
                        if tire {
                            apply_tire_ocr_text(&ui, &s, &text);
                        } else {
                            apply_receipt_ocr_text(&ui, &s, &text);
                        }
                    } else {
                        ui.set_status_message(
                            "No shared OCR text yet. In OCR app: Share → FixItGarage, or Copy then Paste & fill.".into(),
                        );
                    }
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_parse_receipt_text(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let text = ui.get_rcp_paste().to_string();
                let p = parse_receipt_text(&text);
                let u = state.lock().unwrap().unit_system();
                let mut filled = 0u32;
                if let Some(d) = p.date {
                    ui.set_rcp_date(d.into());
                    filled += 1;
                }
                if let Some(m) = p.mileage {
                    // Receipt text usually miles; convert to display unit
                    ui.set_rcp_mileage(miles_to_display(m, u).to_string().into());
                    filled += 1;
                }
                if let Some(g) = p.gallons {
                    // Receipt text usually gallons
                    let shown = gallons_to_display(g, u);
                    ui.set_rcp_gallons(format!("{shown:.3}").into());
                    filled += 1;
                }
                if let Some(v) = p.parts_cost {
                    ui.set_rcp_parts(format!("{v:.2}").into());
                    filled += 1;
                }
                if let Some(v) = p.labor_cost {
                    ui.set_rcp_labor(format!("{v:.2}").into());
                    filled += 1;
                }
                if let Some(v) = p.fuel_cost {
                    ui.set_rcp_fuel(format!("{v:.2}").into());
                    filled += 1;
                }
                if let Some(s) = p.shop_name {
                    ui.set_rcp_shop(s.into());
                    filled += 1;
                }
                if let Some(t) = p.title {
                    if ui.get_rcp_title().is_empty() {
                        ui.set_rcp_title(t.into());
                        filled += 1;
                    }
                } else if ui.get_rcp_title().is_empty() {
                    ui.set_rcp_title("Receipt import".into());
                }
                ui.set_status_message(format!("Parsed {filled} field(s) from text. Review and save.").into());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_paste_receipt_clipboard(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match read_clipboard() {
                    Some(text) => {
                        ui.set_rcp_paste(text.clone().into());
                        // One-tap: paste + auto-fill in the same action
                        let p = parse_receipt_text(&text);
                        let u = state.lock().unwrap().unit_system();
                        let mut filled = 0u32;
                        if let Some(d) = p.date {
                            ui.set_rcp_date(d.into());
                            filled += 1;
                        }
                        if let Some(m) = p.mileage {
                            ui.set_rcp_mileage(miles_to_display(m, u).to_string().into());
                            filled += 1;
                        }
                        if let Some(g) = p.gallons {
                            let shown = gallons_to_display(g, u);
                            ui.set_rcp_gallons(format!("{shown:.3}").into());
                            filled += 1;
                        }
                        if let Some(v) = p.parts_cost {
                            ui.set_rcp_parts(format!("{v:.2}").into());
                            filled += 1;
                        }
                        if let Some(v) = p.labor_cost {
                            ui.set_rcp_labor(format!("{v:.2}").into());
                            filled += 1;
                        }
                        if let Some(v) = p.fuel_cost {
                            ui.set_rcp_fuel(format!("{v:.2}").into());
                            filled += 1;
                        }
                        if let Some(s) = p.shop_name {
                            ui.set_rcp_shop(s.into());
                            filled += 1;
                        }
                        if let Some(t) = p.title {
                            if ui.get_rcp_title().is_empty() {
                                ui.set_rcp_title(t.into());
                                filled += 1;
                            }
                        } else if ui.get_rcp_title().is_empty() {
                            ui.set_rcp_title("Receipt import".into());
                        }
                        ui.set_status_message(format!(
                            "Clipboard → {filled} field(s) filled. Review and save."
                        ).into());
                    }
                    None => ui.set_status_message("Clipboard empty or unavailable.".into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_paste_tire_clipboard(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match read_clipboard() {
                    Some(text) => {
                        ui.set_tire_rcp_paste(text.clone().into());
                        let p = parse_tire_receipt_text(&text);
                        let u = state.lock().unwrap().unit_system();
                        let mut filled = 0u32;
                        if let Some(b) = p.brand {
                            ui.set_tire_brand(b.into());
                            filled += 1;
                        }
                        if let Some(m) = p.model {
                            ui.set_tire_model(m.into());
                            filled += 1;
                        }
                        if let Some(sz) = p.size {
                            ui.set_tire_size(sz.into());
                            filled += 1;
                        }
                        if let Some(c) = p.cost {
                            ui.set_tire_cost(format!("{c:.2}").into());
                            filled += 1;
                        }
                        if let Some(mi) = p.mileage {
                            ui.set_tire_buy_mileage(miles_to_display(mi, u).to_string().into());
                            filled += 1;
                        }
                        if let Some(n) = p.notes {
                            if ui.get_tire_buy_notes().is_empty() {
                                ui.set_tire_buy_notes(n.into());
                            }
                        }
                        ui.set_status_message(format!(
                            "Clipboard → tire receipt ({filled} fields). Review and Save."
                        ).into());
                    }
                    None => ui.set_status_message("Clipboard empty or unavailable.".into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_open_ocr_helper(move || {
            if let Some(ui) = ui_weak.upgrade() {
                if ui.get_page() == 3 {
                    open_ocr_helper_for_tire();
                } else {
                    open_ocr_helper();
                }
            } else {
                open_ocr_helper();
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_cloud_settings(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.lock().unwrap();
                let pass = ui.get_cloud_pass().to_string();
                match s.set_cloud_settings(
                    ui.get_cloud_url().to_string(),
                    ui.get_cloud_user().to_string(),
                    pass,
                ) {
                    Ok(()) => {
                        // Never leave password sitting in the text field after save.
                        ui.set_cloud_pass("".into());
                        let kept = if s.cloud_password.is_empty() {
                            "No password stored yet."
                        } else {
                            "Password kept on device only (not shown again)."
                        };
                        ui.set_status_message(
                            format!("Cloud settings saved. {kept} Leave password blank to keep existing.")
                                .into(),
                        );
                        refresh_ui(&ui, &s);
                    }
                    Err(e) => ui.set_status_message(format!("Cloud settings: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_upload_cloud_backup(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let s = state.lock().unwrap();
                match s.write_backup_file() {
                    Ok(path) => match std::fs::read(&path) {
                        Ok(bytes) => {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("fixitgarage-backup.json");
                            match webdav::upload_backup(
                                &s.cloud_webdav_url,
                                &s.cloud_username,
                                &s.cloud_password,
                                name,
                                &bytes,
                            ) {
                                Ok(msg) => ui.set_status_message(msg.into()),
                                Err(e) => ui.set_status_message(format!("Cloud upload failed: {e}").into()),
                            }
                        }
                        Err(e) => ui.set_status_message(format!("Read backup failed: {e}").into()),
                    },
                    Err(e) => ui.set_status_message(format!("Backup failed: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_service_search(move |q| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_service_search(q);
                refresh_ui(&ui, &state.lock().unwrap());
            }
        });
    }

    ui.on_open_feedback(|| {
        open_url("https://github.com/linuxbased79/FixItGarage/issues");
    });
    ui.on_open_donate(|| {
        // Project website donate page (Liberapay / Sponsors / Ko-fi / PayPal)
        open_url("https://linuxbased79.github.io/FixItGarage/donate.html");
    });

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_create_seller_packet(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let s = state.lock().unwrap();
                match seller_packet::build_seller_packet(&s) {
                    Ok((path, text)) => {
                        // Also write text alongside PDF
                        let txt_path = path.with_extension("txt");
                        let _ = std::fs::write(&txt_path, &text);
                        ui.set_status_message(
                            format!(
                                "Seller packet ready: {}. Sharing PDF…",
                                path.display()
                            )
                            .into(),
                        );
                        let subject = format!(
                            "Maintenance packet — {}",
                            s.selected_vehicle()
                                .map(|v| v.name.as_str())
                                .unwrap_or("vehicle")
                        );
                        if let Err(e) = share_file(
                            &subject,
                            &path.display().to_string(),
                            "application/pdf",
                        ) {
                            // Fallback: share text summary
                            share_text(&subject, &text);
                            ui.set_status_message(
                                format!("PDF saved ({}). Share PDF failed ({e}); shared text summary.", path.display())
                                    .into(),
                            );
                        }
                    }
                    Err(e) => ui.set_status_message(format!("Seller packet: {e}").into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_share_seller_summary(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let s = state.lock().unwrap();
                match seller_packet::build_seller_packet(&s) {
                    Ok((path, text)) => {
                        let subject = format!(
                            "Maintenance packet — {}",
                            s.selected_vehicle()
                                .map(|v| v.name.as_str())
                                .unwrap_or("vehicle")
                        );
                        share_text(&subject, &text);
                        ui.set_status_message(
                            format!(
                                "Seller summary shared. PDF also at {}",
                                path.display()
                            )
                            .into(),
                        );
                    }
                    Err(e) => ui.set_status_message(format!("Seller packet: {e}").into()),
                }
            }
        });
    }

    ui.run()
}

fn truncate_ui(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        let cut: String = t.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Consume ShareReceiveActivity text / pending image and auto-fill the form.
/// Returns true if shared OCR text was applied.
fn try_consume_pending_ocr(ui: &MainWindow, state: &AppState, tire: bool) -> bool {
    if let Some(img) = pending_ocr_image_path() {
        if !tire {
            ui.set_rcp_photo_path(img.into());
        }
    }
    if let Some(text) = take_pending_ocr_text() {
        if tire || ocr_target() == "tire" {
            apply_tire_ocr_text(ui, state, &text);
        } else {
            apply_receipt_ocr_text(ui, state, &text);
        }
        return true;
    }
    false
}

fn apply_receipt_ocr_text(ui: &MainWindow, state: &AppState, text: &str) {
    ui.set_rcp_paste(text.into());
    let p = parse_receipt_text(text);
    let u = state.unit_system();
    let mut filled = 0u32;
    if let Some(d) = p.date {
        ui.set_rcp_date(d.into());
        filled += 1;
    }
    if let Some(m) = p.mileage {
        ui.set_rcp_mileage(miles_to_display(m, u).to_string().into());
        filled += 1;
    }
    if let Some(g) = p.gallons {
        let shown = gallons_to_display(g, u);
        ui.set_rcp_gallons(format!("{shown:.3}").into());
        filled += 1;
    }
    if let Some(v) = p.parts_cost {
        ui.set_rcp_parts(format!("{v:.2}").into());
        filled += 1;
    }
    if let Some(v) = p.labor_cost {
        ui.set_rcp_labor(format!("{v:.2}").into());
        filled += 1;
    }
    if let Some(v) = p.fuel_cost {
        ui.set_rcp_fuel(format!("{v:.2}").into());
        filled += 1;
    }
    if let Some(s) = p.shop_name {
        ui.set_rcp_shop(s.into());
        filled += 1;
    }
    if let Some(t) = p.title {
        if ui.get_rcp_title().is_empty() {
            ui.set_rcp_title(t.into());
            filled += 1;
        }
    } else if ui.get_rcp_title().is_empty() {
        ui.set_rcp_title("Receipt import".into());
    }
    ui.set_status_message(
        format!("OCR text applied — {filled} field(s). Review and save.").into(),
    );
}

fn apply_title_ocr_text(ui: &MainWindow, state: &Arc<Mutex<AppState>>, text: &str) {
    let fields = title_parse::parse_title_text(text);
    let mut filled = Vec::new();
    if let Some(vin) = fields.vin {
        ui.set_form_vin(vin.into());
        filled.push("VIN");
    }
    if let Some(year) = fields.year {
        ui.set_form_year(year.to_string().into());
        filled.push("year");
    }
    if let Some(make) = fields.make {
        ui.set_form_make(make.into());
        filled.push("make");
    }
    if let Some(model) = fields.model {
        ui.set_form_model(model.into());
        filled.push("model");
    }
    if let Some(name) = fields.name_hint {
        if ui.get_form_name().is_empty() {
            ui.set_form_name(name.into());
            filled.push("name");
        }
    }
    // Persist immediately so fields are not lost if the user leaves the screen.
    ensure_vehicle_from_form(ui, state);
    let s = state.lock().unwrap();
    let ok = s.save();
    fill_form_from_selected(ui, &s);
    refresh_ui(ui, &s);
    if filled.is_empty() {
        ui.set_status_message(
            "OCR finished but no VIN/year/make/model found. Try a clearer photo of the full title (good light, flat)."
                .into(),
        );
    } else if ok {
        ui.set_status_message(
            format!(
                "Filled from title: {}. Saved on this device — review fields, tap Save if you edit more.",
                filled.join(", ")
            )
            .into(),
        );
    } else {
        ui.set_status_message(
            format!(
                "Filled from title: {}. WARNING: save may have failed — tap Save vehicle.",
                filled.join(", ")
            )
            .into(),
        );
    }
}

fn apply_tire_ocr_text(ui: &MainWindow, state: &AppState, text: &str) {
    ui.set_tire_rcp_paste(text.into());
    let p = parse_tire_receipt_text(text);
    let u = state.unit_system();
    let mut filled = 0u32;
    if let Some(b) = p.brand {
        ui.set_tire_brand(b.into());
        filled += 1;
    }
    if let Some(m) = p.model {
        ui.set_tire_model(m.into());
        filled += 1;
    }
    if let Some(sz) = p.size {
        ui.set_tire_size(sz.into());
        filled += 1;
    }
    if let Some(c) = p.cost {
        ui.set_tire_cost(format!("{c:.2}").into());
        filled += 1;
    }
    if let Some(mi) = p.mileage {
        ui.set_tire_buy_mileage(miles_to_display(mi, u).to_string().into());
        filled += 1;
    }
    if let Some(n) = p.notes {
        if ui.get_tire_buy_notes().is_empty() {
            ui.set_tire_buy_notes(n.into());
        }
    }
    ui.set_status_message(
        format!("OCR tire text applied — {filled} field(s). Review and Save.").into(),
    );
}

fn refresh_ui(ui: &MainWindow, state: &AppState) {
    ui.set_wizard_done(state.wizard_done);
    ui.set_user_mode(state.user_mode.clone().into());
    let dark = state.dark_mode.eq_ignore_ascii_case("DARK");
    ui.set_dark_mode(if dark { "DARK" } else { "LIGHT" }.into());
    Theme::get(ui).set_dark(dark);
    ui.set_dyslexia_font(state.dyslexia_font);
    Theme::get(ui).set_dyslexia_font(state.dyslexia_font);

    let flags = state.feature_flags();
    ui.set_show_tires(flags.show_tires);
    ui.set_show_parts(flags.show_parts);
    ui.set_show_diy_trackers(flags.show_diy_trackers);
    ui.set_show_shop_emphasis(flags.show_shop_emphasis);

    // Language packs first so labels below use the chosen pack
    let lang_pref = state.language_pref();
    let lang = resolve_lang(lang_pref, &system_locale());
    ui.set_language(lang_pref.as_str().into());
    let pref_label = match lang_pref {
        LanguagePref::System => t(lang, "settings.lang_system"),
        LanguagePref::En => t(lang, "settings.lang_en"),
        LanguagePref::Es => t(lang, "settings.lang_es"),
        LanguagePref::Fr => t(lang, "settings.lang_fr"),
        LanguagePref::De => t(lang, "settings.lang_de"),
        LanguagePref::Ja => t(lang, "settings.lang_ja"),
        LanguagePref::Ko => t(lang, "settings.lang_ko"),
        LanguagePref::Zh => t(lang, "settings.lang_zh"),
    };
    ui.set_language_active_label(format!("{pref_label} → {}", lang.code()).into());

    ui.set_mode_label(state.mode_label().into());
    ui.set_vehicle_count_label(
        t(lang, "vehicle.count")
            .replace("{n}", &state.vehicles.len().to_string())
            .into(),
    );

    let sel_name = state
        .selected_vehicle()
        .map(|v| v.name.clone())
        .unwrap_or_else(|| t(lang, "vehicle.none"));
    ui.set_selected_vehicle_label(sel_name.into());
    ui.set_selected_vehicle_id(state.selected_vehicle_id.unwrap_or(0) as i32);

    // Keep form fields in sync with the selected vehicle so a restart doesn't look
    // like "vehicle disappeared" (list had it, edit fields were empty).
    if state.selected_vehicle_id.is_some() {
        // Only auto-fill when form is empty OR already matches selection by name/vin
        // — avoid stomping mid-edit of a brand-new car the user is typing.
        let form_empty = ui.get_form_name().is_empty()
            && ui.get_form_vin().is_empty()
            && ui.get_form_make().is_empty()
            && ui.get_form_model().is_empty();
        if form_empty {
            fill_form_from_selected(ui, state);
        }
    }

    let u = state.unit_system();
    ui.set_units(u.as_str().into());
    ui.set_label_distance(u.distance_label().into());
    ui.set_label_fuel(u.fuel_label().into());
    ui.set_label_economy(u.economy_unit().into());
    ui.set_label_tread(u.tread_unit().into());
    ui.set_unit_distance(u.distance_unit().into());
    ui.set_unit_fuel(u.fuel_unit().into());

    ui.set_nav_home(t(lang, "nav.home").into());
    ui.set_nav_cars(t(lang, "nav.cars").into());
    ui.set_nav_service(t(lang, "nav.service").into());
    ui.set_nav_tires(t(lang, "nav.tires").into());
    ui.set_nav_costs(t(lang, "nav.costs").into());
    ui.set_nav_more(t(lang, "nav.more").into());
    ui.set_nav_settings(t(lang, "nav.settings").into());
    ui.set_tr_settings_title(t(lang, "settings.title").into());
    ui.set_tr_settings_intro(t(lang, "settings.intro").into());
    ui.set_tr_settings_appearance(t(lang, "settings.appearance").into());
    ui.set_tr_settings_appearance_body(t(lang, "settings.appearance_body").into());
    ui.set_tr_settings_dark(t(lang, "settings.dark").into());
    ui.set_tr_settings_light(t(lang, "settings.light").into());
    ui.set_tr_settings_units(t(lang, "settings.units").into());
    ui.set_tr_settings_units_body(t(lang, "settings.units_body").into());
    ui.set_tr_settings_imperial(t(lang, "settings.imperial").into());
    ui.set_tr_settings_metric(t(lang, "settings.metric").into());
    ui.set_tr_settings_language(t(lang, "settings.language").into());
    ui.set_tr_settings_language_body(t(lang, "settings.language_body").into());
    ui.set_tr_settings_lang_system(t(lang, "settings.lang_system").into());
    ui.set_tr_settings_feature(t(lang, "settings.feature_focus").into());
    ui.set_tr_settings_feature_body(t(lang, "settings.feature_body").into());
    ui.set_tr_settings_data(t(lang, "settings.data").into());
    ui.set_tr_settings_data_body(t(lang, "settings.data_body").into());
    ui.set_tr_settings_cloud(t(lang, "settings.cloud").into());
    ui.set_tr_settings_webdav(t(lang, "settings.webdav").into());
    ui.set_tr_settings_support(t(lang, "settings.support").into());
    ui.set_tr_settings_about(t(lang, "settings.about").into());
    ui.set_tr_settings_donate(t(lang, "settings.donate").into());
    ui.set_tr_settings_feedback(t(lang, "settings.feedback").into());
    ui.set_tr_more_title(t(lang, "more.title").into());
    ui.set_tr_more_intro(t(lang, "more.intro").into());
    ui.set_tr_more_trackers(t(lang, "more.trackers").into());
    ui.set_tr_more_logs(t(lang, "more.logs").into());
    ui.set_tr_more_quick(t(lang, "more.quick").into());
    ui.set_tr_more_open_settings(t(lang, "more.open_settings").into());
    ui.set_tr_home_last_service(t(lang, "home.last_service").into());
    ui.set_tr_home_vehicles(t(lang, "home.vehicles").into());
    ui.set_tr_home_quick(t(lang, "home.quick_actions").into());
    ui.set_tr_home_glance(t(lang, "home.at_a_glance").into());
    ui.set_tr_home_upcoming(t(lang, "home.upcoming").into());
    ui.set_tr_app_title(t(lang, "app.title").into());
    ui.set_tr_home_due(t(lang, "home.due_reminders").into());
    ui.set_tr_home_tread(t(lang, "home.tread_warning").into());
    ui.set_tr_home_no_upcoming(t(lang, "home.no_upcoming").into());
    ui.set_tr_home_open_reminders(t(lang, "home.open_reminders").into());
    ui.set_tr_home_manage_vehicles(t(lang, "home.manage_vehicles").into());
    ui.set_tr_home_log_service(t(lang, "home.log_service").into());
    ui.set_tr_home_fuel(t(lang, "home.fuel_history").into());
    ui.set_tr_home_tire_rotation(t(lang, "home.tire_rotation").into());
    ui.set_tr_page_vehicles(t(lang, "page.vehicles").into());
    ui.set_tr_page_service(t(lang, "page.service").into());
    ui.set_tr_page_tires(t(lang, "page.tires").into());
    ui.set_tr_page_costs(t(lang, "page.costs").into());
    ui.set_tr_page_parts(t(lang, "page.parts").into());
    ui.set_tr_page_battery(t(lang, "page.battery").into());
    ui.set_tr_page_wipers(t(lang, "page.wipers").into());
    ui.set_tr_page_brakes(t(lang, "page.brakes").into());
    ui.set_tr_page_notes(t(lang, "page.notes").into());
    ui.set_tr_page_reminders(t(lang, "page.reminders").into());
    ui.set_tr_page_photos(t(lang, "page.photos").into());
    ui.set_tr_page_receipts(t(lang, "page.receipts").into());
    ui.set_tr_page_fuel(t(lang, "page.fuel").into());
    ui.set_tr_more_parts(t(lang, "more.parts").into());
    ui.set_tr_more_battery(t(lang, "more.battery").into());
    ui.set_tr_more_wipers(t(lang, "more.wipers").into());
    ui.set_tr_more_brakes(t(lang, "more.brakes").into());
    ui.set_tr_more_notes(t(lang, "more.notes").into());
    ui.set_tr_more_reminders(t(lang, "more.reminders").into());
    ui.set_tr_more_photos(t(lang, "more.photos").into());
    ui.set_tr_more_receipts(t(lang, "more.receipts").into());
    ui.set_tr_more_fuel(t(lang, "more.fuel").into());
    ui.set_tr_more_log_service(t(lang, "more.log_service").into());
    ui.set_tr_more_tire_tracker(t(lang, "more.tire_tracker").into());
    ui.set_tr_more_selling(t(lang, "more.selling").into());
    ui.set_tr_more_selling_body(t(lang, "more.selling_body").into());
    ui.set_tr_more_seller_pdf(t(lang, "more.seller_pdf").into());
    ui.set_tr_more_seller_text(t(lang, "more.seller_text").into());
    ui.set_tr_common_switch(t(lang, "common.switch").into());
    ui.set_tr_common_vehicle(t(lang, "common.vehicle").into());
    ui.set_tr_common_back_more(t(lang, "common.back_more").into());
    ui.set_tr_common_delete(t(lang, "common.delete").into());
    // Service page
    ui.set_tr_svc_search_ph(t(lang, "service.search_ph").into());
    ui.set_tr_svc_quick_templates(t(lang, "service.quick_templates").into());
    ui.set_tr_svc_quick_templates_body(t(lang, "service.quick_templates_body").into());
    ui.set_tr_svc_oil_change(t(lang, "service.oil_change").into());
    ui.set_tr_svc_fuel_fill(t(lang, "service.fuel_fill").into());
    ui.set_tr_svc_shop_visit(t(lang, "service.shop_visit").into());
    ui.set_tr_svc_log_title(t(lang, "service.log_title").into());
    ui.set_tr_svc_log_body(
        if flags.show_shop_emphasis {
            t(lang, "service.log_body_shop")
        } else {
            t(lang, "service.log_body_diy")
        }
        .into(),
    );
    ui.set_tr_svc_title_ph(t(lang, "service.title_ph").into());
    ui.set_tr_svc_diy(t(lang, "service.diy").into());
    ui.set_tr_svc_shop(t(lang, "service.shop").into());
    ui.set_tr_svc_parts_cost(t(lang, "service.parts_cost").into());
    ui.set_tr_svc_labor_cost(t(lang, "service.labor_cost").into());
    ui.set_tr_svc_fuel_optional(t(lang, "service.fuel_optional").into());
    ui.set_tr_svc_fuel_cost_ph(t(lang, "service.fuel_cost_ph").into());
    ui.set_tr_svc_shop_name_ph(t(lang, "service.shop_name_ph").into());
    ui.set_tr_svc_notes_ph(t(lang, "service.notes_ph").into());
    ui.set_tr_svc_save(t(lang, "service.save").into());
    ui.set_tr_common_back_home(t(lang, "common.back_home").into());
    ui.set_tr_common_brand(t(lang, "common.brand").into());
    ui.set_tr_common_cost(t(lang, "common.cost").into());
    ui.set_tr_common_dark(t(lang, "common.dark").into());
    ui.set_tr_common_date_ymd(t(lang, "common.date_ymd").into());
    ui.set_tr_common_imperial(t(lang, "common.imperial").into());
    ui.set_tr_common_light(t(lang, "common.light").into());
    ui.set_tr_common_metric(t(lang, "common.metric").into());
    ui.set_tr_common_notes(t(lang, "common.notes").into());
    ui.set_tr_common_notes_opt(t(lang, "common.notes_opt").into());
    ui.set_tr_common_save(t(lang, "common.save").into());
    ui.set_tr_common_title(t(lang, "common.title").into());
    ui.set_tr_common_update(t(lang, "common.update").into());
    ui.set_tr_costs_all_time(t(lang, "costs.all_time").into());
    ui.set_tr_costs_body(t(lang, "costs.body").into());
    ui.set_tr_costs_month(t(lang, "costs.month").into());
    ui.set_tr_costs_services(t(lang, "costs.services").into());
    ui.set_tr_costs_tires(t(lang, "costs.tires").into());
    ui.set_tr_costs_year(t(lang, "costs.year").into());
    ui.set_tr_fuel_each_fill(t(lang, "fuel.each_fill").into());
    ui.set_tr_fuel_log_in_service(t(lang, "fuel.log_in_service").into());
    ui.set_tr_home_issue_photos(t(lang, "home.issue_photos").into());
    ui.set_tr_home_manage_reminders(t(lang, "home.manage_reminders").into());
    ui.set_tr_home_notes(t(lang, "home.notes").into());
    ui.set_tr_home_oil_level(t(lang, "home.oil_level").into());
    ui.set_tr_home_open_battery(t(lang, "home.open_battery").into());
    ui.set_tr_home_open_brakes(t(lang, "home.open_brakes").into());
    ui.set_tr_home_open_tires(t(lang, "home.open_tires").into());
    ui.set_tr_home_open_wipers(t(lang, "home.open_wipers").into());
    ui.set_tr_home_parts_log(t(lang, "home.parts_log").into());
    ui.set_tr_home_reminders(t(lang, "home.reminders").into());
    ui.set_tr_home_scan_receipt(t(lang, "home.scan_receipt").into());
    ui.set_tr_notes_body(t(lang, "notes.body").into());
    ui.set_tr_notes_new(t(lang, "notes.new").into());
    ui.set_tr_notes_save(t(lang, "notes.save").into());
    ui.set_tr_photos_caption_ph(t(lang, "photos.caption_ph").into());
    ui.set_tr_photos_new(t(lang, "photos.new").into());
    ui.set_tr_photos_save(t(lang, "photos.save").into());
    ui.set_tr_photos_take(t(lang, "photos.take").into());
    ui.set_tr_receipt_apply_shared(t(lang, "receipt.apply_shared").into());
    ui.set_tr_receipt_auto_fill(t(lang, "receipt.auto_fill").into());
    ui.set_tr_receipt_capture(t(lang, "receipt.capture").into());
    ui.set_tr_receipt_fields(t(lang, "receipt.fields").into());
    ui.set_tr_receipt_hint(t(lang, "receipt.hint").into());
    ui.set_tr_receipt_ocr_helper(t(lang, "receipt.ocr_helper").into());
    ui.set_tr_receipt_ondevice_ocr(t(lang, "receipt.ondevice_ocr").into());
    ui.set_tr_receipt_parse(t(lang, "receipt.parse").into());
    ui.set_tr_receipt_paste_ph(t(lang, "receipt.paste_ph").into());
    ui.set_tr_receipt_save(t(lang, "receipt.save").into());
    ui.set_tr_receipt_send_ocr(t(lang, "receipt.send_ocr").into());
    ui.set_tr_receipt_title_ph(t(lang, "receipt.title_ph").into());
    ui.set_tr_rem_add(t(lang, "rem.add").into());
    ui.set_tr_rem_add_body(t(lang, "rem.add_body").into());
    ui.set_tr_rem_due_date(t(lang, "rem.due_date").into());
    ui.set_tr_rem_reboot_hint(t(lang, "rem.reboot_hint").into());
    ui.set_tr_rem_save(t(lang, "rem.save").into());
    ui.set_tr_rem_title(t(lang, "rem.title").into());
    ui.set_tr_rem_mark_done(t(lang, "rem.mark_done").into());
    ui.set_tr_rem_log_mark_done(t(lang, "rem.log_mark_done").into());
    ui.set_tr_oil_intro(t(lang, "oil.intro").into());
    ui.set_tr_oil_dipstick(t(lang, "oil.dipstick").into());
    ui.set_tr_oil_pick(t(lang, "oil.pick").into());
    ui.set_tr_oil_selected(t(lang, "oil.selected").into());
    ui.set_tr_oil_will_log(t(lang, "oil.will_log").into());
    ui.set_tr_tires_four_only(t(lang, "tires.four_only").into());
    ui.set_tr_tires_pat_fwd(t(lang, "tires.pat_fwd").into());
    ui.set_tr_tires_pat_x(t(lang, "tires.pat_x").into());
    ui.set_tr_tires_pat_rear(t(lang, "tires.pat_rear").into());
    ui.set_tr_tires_pat_side(t(lang, "tires.pat_side").into());
    ui.set_tr_tires_patterns_4(t(lang, "tires.patterns_4").into());
    ui.set_tr_tires_patterns_5(t(lang, "tires.patterns_5").into());
    ui.set_tr_tires_tread_moves(t(lang, "tires.tread_moves").into());
    ui.set_tr_tires_spare_id_ph(t(lang, "tires.spare_id_ph").into());
    ui.set_tr_tires_five_cycle(t(lang, "tires.five_cycle").into());
    ui.set_tr_wipers_intro(t(lang, "wipers.intro").into());
    ui.set_tr_wipers_driver(t(lang, "wipers.driver").into());
    ui.set_tr_wipers_passenger(t(lang, "wipers.passenger").into());
    ui.set_tr_wipers_rear(t(lang, "wipers.rear").into());
    ui.set_tr_wipers_update(t(lang, "wipers.update").into());
    ui.set_tr_wipers_update_body(t(lang, "wipers.update_body").into());
    ui.set_tr_wipers_driver_btn(t(lang, "wipers.driver_btn").into());
    ui.set_tr_wipers_passenger_btn(t(lang, "wipers.passenger_btn").into());
    ui.set_tr_wipers_size_ph(t(lang, "wipers.size_ph").into());
    ui.set_tr_wipers_install_date_ph(t(lang, "wipers.install_date_ph").into());
    ui.set_tr_wipers_at_install(t(lang, "wipers.at_install").into());
    ui.set_tr_wipers_save(t(lang, "wipers.save").into());
    ui.set_tr_brakes_intro(t(lang, "brakes.intro").into());
    ui.set_tr_brakes_front_pads(t(lang, "brakes.front_pads").into());
    ui.set_tr_brakes_rear_pads(t(lang, "brakes.rear_pads").into());
    ui.set_tr_brakes_fluid(t(lang, "brakes.fluid").into());
    ui.set_tr_brakes_fr_pads(t(lang, "brakes.fr_pads").into());
    ui.set_tr_brakes_rr_pads(t(lang, "brakes.rr_pads").into());
    ui.set_tr_brakes_fluid_btn(t(lang, "brakes.fluid_btn").into());
    ui.set_tr_brakes_save(t(lang, "brakes.save").into());
    ui.set_tr_parts_current(t(lang, "parts.current").into());
    ui.set_tr_parts_air(t(lang, "parts.air").into());
    ui.set_tr_parts_cabin(t(lang, "parts.cabin").into());
    ui.set_tr_parts_oil_filt(t(lang, "parts.oil_filt").into());
    ui.set_tr_parts_oil_filter(t(lang, "parts.oil_filter").into());
    ui.set_tr_parts_oil_type(t(lang, "parts.oil_type").into());
    ui.set_tr_parts_save_update(t(lang, "parts.save_update").into());
    ui.set_tr_parts_save_body(t(lang, "parts.save_body").into());
    ui.set_tr_parts_part_number(t(lang, "parts.part_number").into());
    ui.set_tr_parts_viscosity(t(lang, "parts.viscosity").into());
    ui.set_tr_parts_installed(t(lang, "parts.installed").into());
    ui.set_tr_parts_save(t(lang, "parts.save").into());
    ui.set_tr_batt_current(t(lang, "batt.current").into());
    ui.set_tr_batt_age(t(lang, "batt.age").into());
    ui.set_tr_batt_update(t(lang, "batt.update").into());
    ui.set_tr_batt_date_hint(t(lang, "batt.date_hint").into());
    ui.set_tr_batt_install_ph(t(lang, "batt.install_ph").into());
    ui.set_tr_batt_notes_ph(t(lang, "batt.notes_ph").into());
    ui.set_tr_batt_save(t(lang, "batt.save").into());
    ui.set_tr_photos_intro(t(lang, "photos.intro").into());
    ui.set_tr_photos_notes_ph(t(lang, "photos.notes_ph").into());
    ui.set_tr_photos_no_path(t(lang, "photos.no_path").into());
    ui.set_tr_photos_photo_prefix(t(lang, "photos.photo_prefix").into());
    ui.set_tr_receipt_fields_body(t(lang, "receipt.fields_body").into());
    ui.set_tr_receipt_no_photo(t(lang, "receipt.no_photo").into());
    ui.set_tr_receipt_open_ocr(t(lang, "receipt.open_ocr").into());
    ui.set_tr_receipt_quick_camera(t(lang, "receipt.quick_camera").into());
    ui.set_tr_receipt_fuel_suffix(t(lang, "receipt.fuel_suffix").into());
    ui.set_tr_receipt_shop_ph(t(lang, "receipt.shop_ph").into());
    ui.set_tr_settings_about_body(t(lang, "settings.about_body").into());
    ui.set_tr_settings_accessibility(t(lang, "settings.accessibility").into());
    ui.set_tr_settings_accessibility_body(t(lang, "settings.accessibility_body").into());
    ui.set_tr_settings_backup_path_ph(t(lang, "settings.backup_path_ph").into());
    ui.set_tr_settings_cloud_body(t(lang, "settings.cloud_body").into());
    ui.set_tr_settings_create_backup(t(lang, "settings.create_backup").into());
    ui.set_tr_settings_default_font(t(lang, "settings.default_font").into());
    ui.set_tr_settings_dropbox(t(lang, "settings.dropbox").into());
    ui.set_tr_settings_export_csv(t(lang, "settings.export_csv").into());
    ui.set_tr_settings_google_drive(t(lang, "settings.google_drive").into());
    ui.set_tr_settings_gpl(t(lang, "settings.gpl").into());
    ui.set_tr_settings_onedrive(t(lang, "settings.onedrive").into());
    ui.set_tr_settings_password_ph(t(lang, "settings.password_ph").into());
    ui.set_tr_settings_proton(t(lang, "settings.proton").into());
    ui.set_tr_settings_restore(t(lang, "settings.restore").into());
    ui.set_tr_settings_save_webdav(t(lang, "settings.save_webdav").into());
    ui.set_tr_settings_share_backup(t(lang, "settings.share_backup").into());
    ui.set_tr_settings_share_csv(t(lang, "settings.share_csv").into());
    ui.set_tr_settings_upload_webdav(t(lang, "settings.upload_webdav").into());
    ui.set_tr_settings_username(t(lang, "settings.username").into());
    ui.set_tr_settings_webdav_hint(t(lang, "settings.webdav_hint").into());
    ui.set_tr_settings_webdav_url_ph(t(lang, "settings.webdav_url_ph").into());
    ui.set_tr_tires_after_preview(t(lang, "tires.after_preview").into());
    ui.set_tr_tires_apply_rotation(t(lang, "tires.apply_rotation").into());
    ui.set_tr_tires_camera_coin(t(lang, "tires.camera_coin").into());
    ui.set_tr_tires_current_before(t(lang, "tires.current_before").into());
    ui.set_tr_tires_distance(t(lang, "tires.distance").into());
    ui.set_tr_tires_front_view(t(lang, "tires.front_view").into());
    ui.set_tr_tires_hidden(t(lang, "tires.hidden").into());
    ui.set_tr_tires_include_spare(t(lang, "tires.include_spare").into());
    ui.set_tr_tires_measure_cv(t(lang, "tires.measure_cv").into());
    ui.set_tr_tires_paste_fill(t(lang, "tires.paste_fill").into());
    ui.set_tr_tires_paste_receipt(t(lang, "tires.paste_receipt").into());
    ui.set_tr_tires_positions_hint(t(lang, "tires.positions_hint").into());
    ui.set_tr_tires_purchase(t(lang, "tires.purchase").into());
    ui.set_tr_tires_purchase_body(t(lang, "tires.purchase_body").into());
    ui.set_tr_tires_purchase_history(t(lang, "tires.purchase_history").into());
    ui.set_tr_tires_rotation_history(t(lang, "tires.rotation_history").into());
    ui.set_tr_tires_rotation_pattern(t(lang, "tires.rotation_pattern").into());
    ui.set_tr_tires_save_distance(t(lang, "tires.save_distance").into());
    ui.set_tr_tires_save_purchase(t(lang, "tires.save_purchase").into());
    ui.set_tr_tires_save_tread(t(lang, "tires.save_tread").into());
    ui.set_tr_tires_size_ph(t(lang, "tires.size_ph").into());
    ui.set_tr_tires_spare_body(t(lang, "tires.spare_body").into());
    ui.set_tr_tires_spare_title(t(lang, "tires.spare_title").into());
    ui.set_tr_tires_tread_depth(t(lang, "tires.tread_depth").into());
    ui.set_tr_vehicles_add_body(t(lang, "vehicles.add_body").into());
    ui.set_tr_vehicles_add_title(t(lang, "vehicles.add_title").into());
    ui.set_tr_vehicles_check_recalls(t(lang, "vehicles.check_recalls").into());
    ui.set_tr_vehicles_delete(t(lang, "vehicles.delete").into());
    ui.set_tr_vehicles_edit_title(t(lang, "vehicles.edit_title").into());
    ui.set_tr_vehicles_hint(t(lang, "vehicles.hint").into());
    ui.set_tr_vehicles_make_ph(t(lang, "vehicles.make_ph").into());
    ui.set_tr_vehicles_model_ph(t(lang, "vehicles.model_ph").into());
    ui.set_tr_vehicles_name_ph(t(lang, "vehicles.name_ph").into());
    ui.set_tr_vehicles_open_nhtsa(t(lang, "vehicles.open_nhtsa").into());
    ui.set_tr_vehicles_scan_title(t(lang, "vehicles.scan_title").into());
    ui.set_tr_vehicles_ocr_title(t(lang, "vehicles.ocr_title").into());
    ui.set_tr_vehicles_scan_hint(t(lang, "vehicles.scan_hint").into());
    ui.set_tr_vehicles_recalls_body(t(lang, "vehicles.recalls_body").into());
    ui.set_tr_vehicles_recalls_title(t(lang, "vehicles.recalls_title").into());
    ui.set_tr_vehicles_save(t(lang, "vehicles.save").into());
    ui.set_tr_vehicles_save_changes(t(lang, "vehicles.save_changes").into());
    ui.set_tr_vehicles_selected(t(lang, "vehicles.selected").into());
    ui.set_tr_vehicles_update_odo(t(lang, "vehicles.update_odo").into());
    ui.set_tr_vehicles_vin_opt_ph(t(lang, "vehicles.vin_opt_ph").into());
    ui.set_tr_vehicles_vin_ph(t(lang, "vehicles.vin_ph").into());
    ui.set_tr_vehicles_year_ph(t(lang, "vehicles.year_ph").into());
    ui.set_tr_wizard_both(t(lang, "wizard.both").into());
    ui.set_tr_wizard_diy(t(lang, "wizard.diy").into());
    ui.set_tr_wizard_diy_body(t(lang, "wizard.diy_body").into());
    ui.set_tr_wizard_intro(t(lang, "wizard.intro").into());
    ui.set_tr_wizard_shop(t(lang, "wizard.shop").into());
    ui.set_tr_wizard_shop_body(t(lang, "wizard.shop_body").into());
    ui.set_tr_wizard_units_note(t(lang, "wizard.units_note").into());
    ui.set_tr_wizard_welcome(t(lang, "wizard.welcome").into());
    // Oil level choices for current unit system + language (storage stays English)
    let oil_opts = oil_level_labels(lang, u);
    ui.set_oil_opt_0(oil_opts[0].clone().into());
    ui.set_oil_opt_1(oil_opts[1].clone().into());
    ui.set_oil_opt_2(oil_opts[2].clone().into());
    ui.set_oil_opt_3(oil_opts[3].clone().into());
    ui.set_oil_opt_4(oil_opts[4].clone().into());
    ui.set_oil_opt_5(oil_opts[5].clone().into());

    // Economy line uses translated "select a vehicle" / "avg" / "need fills"
    let economy_unit = u.economy_unit();
    let mpg_label = match state.average_mpg() {
        Some(mpg) => format!("{} {}", format_economy(mpg, u), t(lang, "service.avg")),
        None if state.selected_vehicle_id.is_none() => {
            format!("{economy_unit}: {}", t(lang, "service.select_vehicle"))
        }
        None => format!("{economy_unit}: {}", t(lang, "service.need_fills")),
    };
    ui.set_mpg_label(mpg_label.into());
    let tire_cfg = state.selected_tire_config();
    ui.set_tire_fl(tire_cfg.layout.fl.clone().into());
    ui.set_tire_fr(tire_cfg.layout.fr.clone().into());
    ui.set_tire_rl(tire_cfg.layout.rl.clone().into());
    ui.set_tire_rr(tire_cfg.layout.rr.clone().into());
    ui.set_tire_spare(tire_cfg.layout.spare.clone().into());
    ui.set_include_spare(tire_cfg.include_spare);
    let after = state.preview_after_layout();
    ui.set_tire_after_fl(after.fl.into());
    ui.set_tire_after_fr(after.fr.into());
    ui.set_tire_after_rl(after.rl.into());
    ui.set_tire_after_rr(after.rr.into());
    ui.set_tire_after_spare(after.spare.into());
    ui.set_tire_pattern(tire_cfg.pattern.clone().into());
    ui.set_tire_preview(tire_preview_i18n(state, lang).into());

    // Tracker summaries (localized empty/labels)
    ui.set_part_air(part_summary_i18n(state, "ENGINE_AIR_FILTER", lang).into());
    ui.set_part_cabin(part_summary_i18n(state, "CABIN_FILTER", lang).into());
    ui.set_part_oil_filter(part_summary_i18n(state, "OIL_FILTER", lang).into());
    ui.set_part_oil_type(part_summary_i18n(state, "OIL_TYPE", lang).into());
    ui.set_sum_battery(component_summary_i18n(state, "BATTERY", lang).into());
    // LHD convention: driver = left, passenger = right (most US / GrapheneOS devices)
    ui.set_sum_wiper_driver(component_summary_i18n(state, "WIPER_DRIVER", lang).into());
    ui.set_sum_wiper_passenger(component_summary_i18n(state, "WIPER_PASSENGER", lang).into());
    ui.set_sum_wiper_rear(component_summary_i18n(state, "WIPER_REAR", lang).into());
    ui.set_sum_brake_f(component_summary_i18n(state, "BRAKE_PADS_FRONT", lang).into());
    ui.set_sum_brake_r(component_summary_i18n(state, "BRAKE_PADS_REAR", lang).into());
    ui.set_sum_brake_fluid(component_summary_i18n(state, "BRAKE_FLUID", lang).into());

    if let Some(last) = state.last_service() {
        ui.set_last_service_title(last.title.clone().into());
        ui.set_last_service_detail(
            format!(
                "{} · {} · {}",
                format_service_date(last.date_epoch_ms),
                units::format_distance(last.mileage, u),
                last.source.as_str()
            )
            .into(),
        );
    } else {
        ui.set_last_service_title("No services logged yet".into());
        ui.set_last_service_detail("Add a service for the selected vehicle.".into());
    }

    let vehicles: Vec<VehicleRow> = state
        .vehicles
        .iter()
        .map(|v| {
            let selected = state.selected_vehicle_id == Some(v.id);
            let subtitle = format!(
                "{} {} {}",
                v.year.map(|y| y.to_string()).unwrap_or_default(),
                v.make,
                v.model
            )
            .trim()
            .to_string();
            VehicleRow {
                id: v.id as i32,
                name: if selected {
                    format!("✓ {}", v.name).into()
                } else {
                    v.name.clone().into()
                },
                subtitle: subtitle.into(),
                mileage: units::format_distance(v.current_mileage, u).into(),
            }
        })
        .collect();
    ui.set_vehicles(slint::ModelRc::new(slint::VecModel::from(vehicles)));

    let query = ui.get_service_search().to_string();
    let services: Vec<ServiceRow> = state
        .filtered_services(&query)
        .into_iter()
        .map(|s| {
            let total = s.total_cost();
            ServiceRow {
                id: s.id as i32,
                title: s.title.clone().into(),
                subtitle: {
                    let mut d = format!(
                        "{} · {} · {}",
                        format_service_date(s.date_epoch_ms),
                        units::format_distance(s.mileage, u),
                        s.source.as_str()
                    );
                    if !s.notes.trim().is_empty() {
                        d.push_str(&format!(" · {}", s.notes.trim()));
                    }
                    d.into()
                },
                cost: if total > 0.0 {
                    format!("${total:.2}").into()
                } else {
                    "".into()
                },
            }
        })
        .collect();
    ui.set_services(slint::ModelRc::new(slint::VecModel::from(services)));

    let (dash_mpg, dash_month, dash_year, dash_dues) = state.dashboard_lines();
    ui.set_dash_mpg(dash_mpg.into());
    ui.set_dash_month(dash_month.into());
    ui.set_dash_year(dash_year.into());
    ui.set_dash_dues(dash_dues.into());

    let fuel: Vec<HistoryRow> = state
        .fuel_history_lines()
        .into_iter()
        .enumerate()
        .map(|(i, (title, detail))| HistoryRow {
            id: i as i32,
            title: title.into(),
            detail: detail.into(),
        })
        .collect();
    ui.set_fuel_history(slint::ModelRc::new(slint::VecModel::from(fuel)));

    let costs: Vec<CostLine> = state
        .cost_labels()
        .into_iter()
        .map(|(label, value)| CostLine {
            label: label.into(),
            value: value.into(),
        })
        .collect();
    ui.set_cost_lines(slint::ModelRc::new(slint::VecModel::from(costs)));

    let mileage = state
        .selected_vehicle()
        .map(|v| v.current_mileage)
        .unwrap_or(0);
    let notes: Vec<NoteRow> = state
        .notes
        .iter()
        .filter(|n| state.selected_vehicle_id == Some(n.vehicle_id))
        .map(|n| NoteRow {
            id: n.id as i32,
            title: n.title.clone().into(),
            body: n.body.clone().into(),
        })
        .collect();
    ui.set_notes(slint::ModelRc::new(slint::VecModel::from(notes)));

    let reminders: Vec<ReminderRow> = state
        .open_reminders_for_selected()
        .into_iter()
        .map(|r| {
            let is_oil = AppState::is_oil_level_reminder(&r.title);
            ReminderRow {
                id: r.id as i32,
                title: if is_oil {
                    t(lang, "home.oil_level").into()
                } else {
                    r.title.clone().into()
                },
                detail: reminder_status_line_units(r, mileage, u).into(),
                is_oil_level: is_oil,
            }
        })
        .collect();
    ui.set_reminders(slint::ModelRc::new(slint::VecModel::from(reminders)));
    ui.set_last_oil_level_summary(last_oil_level_summary_i18n(state, lang, u).into());
    // Keep a sensible default if empty (canonical English for storage)
    if ui.get_oil_level_choice().is_empty() {
        ui.set_oil_level_choice("Full".into());
    }
    // Localized display of current selection
    let choice = ui.get_oil_level_choice().to_string();
    ui.set_oil_level_display(oil_level_label(lang, u, &choice).into());

    let photos: Vec<PhotoRow> = state
        .issue_photos
        .iter()
        .filter(|p| state.selected_vehicle_id == Some(p.vehicle_id))
        .rev()
        .map(|p| PhotoRow {
            id: p.id as i32,
            title: p.caption.clone().into(),
            detail: format!(
                "{} · {}\n{}",
                format_service_date(p.created_epoch_ms),
                p.file_path,
                p.notes
            )
            .into(),
        })
        .collect();
    ui.set_photos(slint::ModelRc::new(slint::VecModel::from(photos)));

    let purchases: Vec<HistoryRow> = state
        .tire_purchases
        .iter()
        .filter(|p| state.selected_vehicle_id == Some(p.vehicle_id))
        .rev()
        .map(|p| {
            let mi = p
                .mileage
                .map(|m| units::format_distance(m, u))
                .unwrap_or_else(|| "—".into());
            HistoryRow {
                id: p.id as i32,
                title: format!("{} {} {}", p.brand, p.model, p.size)
                    .trim()
                    .to_string()
                    .into(),
                detail: format!(
                    "{} · {} · ${:.2} · {}",
                    format_service_date(p.date_epoch_ms),
                    mi,
                    p.cost,
                    p.notes
                )
                .into(),
            }
        })
        .collect();
    ui.set_tire_purchases(slint::ModelRc::new(slint::VecModel::from(purchases)));

    let rotations: Vec<HistoryRow> = state
        .tire_rotations
        .iter()
        .filter(|r| state.selected_vehicle_id == Some(r.vehicle_id))
        .rev()
        .map(|r| {
            let mi = r
                .mileage
                .map(|m| units::format_distance(m, u))
                .unwrap_or_else(|| "—".into());
            HistoryRow {
                id: r.id as i32,
                title: format!("{} rotation", r.pattern).into(),
                detail: format!(
                    "{} · {} · {}{} / {}{} → {}{} / {}{}",
                    format_service_date(r.date_epoch_ms),
                    mi,
                    r.before_fl,
                    r.before_fr,
                    r.before_rl,
                    r.before_rr,
                    r.after_fl,
                    r.after_fr,
                    r.after_rl,
                    r.after_rr
                )
                .into(),
            }
        })
        .collect();
    ui.set_tire_rotations(slint::ModelRc::new(slint::VecModel::from(rotations)));

    ui.set_due_reminders_banner(state.due_reminders_summary().into());
    ui.set_has_due_reminders(state.has_due_reminders());
    let upcoming: Vec<HistoryRow> = state
        .upcoming_reminders_lines()
        .into_iter()
        .enumerate()
        .map(|(i, (title, detail))| HistoryRow {
            id: i as i32,
            title: title.into(),
            detail: detail.into(),
        })
        .collect();
    ui.set_upcoming_reminders(slint::ModelRc::new(slint::VecModel::from(upcoming)));
    ui.set_tread_summary(state.tread_summary().into());
    ui.set_tread_warning(state.tread_warning().into());
    ui.set_has_low_tread(state.has_low_tread());
    ui.set_battery_warning(state.battery_age_warning().into());
    ui.set_has_old_battery(state.has_old_battery());
    ui.set_brake_warning(state.brake_due_warning().into());
    ui.set_has_brakes_due(state.has_brakes_due());
    ui.set_wiper_warning(state.wiper_due_warning().into());
    ui.set_has_old_wipers(state.has_old_wipers());
    ui.set_tread_coin_guide(state.tread_coin_guide().into());
    ui.set_tire_miles_summary(state.tire_miles_summary().into());
    let tm = state.tire_miles_for_selected();
    ui.set_mi_fl(
        tm.fl
            .map(|v| miles_to_display(v, u).to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_mi_fr(
        tm.fr
            .map(|v| miles_to_display(v, u).to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_mi_rl(
        tm.rl
            .map(|v| miles_to_display(v, u).to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_mi_rr(
        tm.rr
            .map(|v| miles_to_display(v, u).to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_mi_spare(
        tm.spare
            .map(|v| miles_to_display(v, u).to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_cloud_url(state.cloud_webdav_url.clone().into());
    ui.set_cloud_user(state.cloud_username.clone().into());
    // Security: never re-seed WebDAV password into the UI (shoulder-surfing / screenshots).
    // User re-enters only when changing it; blank on save keeps the stored password.
    let t = state.tread_for_selected();
    let fmt_t = |v: f64| {
        let d = mm_to_display(v, u);
        format!("{d:.1}")
    };
    ui.set_tread_fl(t.fl.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_fr(t.fr.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_rl(t.rl.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_rr(t.rr.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_spare(t.spare.map(fmt_t).unwrap_or_default().into());
}

/// Push due notifications (throttled) for all vehicles + selected component alerts.
fn fire_due_notifications(s: &mut AppState) {
    if !s.should_notify_now() {
        return;
    }
    let items = s.all_due_notification_items();
    let mut any = false;
    for (id, title, body) in items.iter().take(8) {
        notify_with_id(*id, title, body);
        any = true;
    }
    // Component alerts for selected vehicle (battery / brakes / wipers / tread)
    if s.has_brakes_due() {
        notify_with_id(42010, "FixItGarage · Brakes", &s.brake_due_warning());
        any = true;
    }
    if s.has_old_battery() {
        notify_with_id(42011, "FixItGarage · Battery", &s.battery_age_warning());
        any = true;
    }
    if s.has_old_wipers() {
        notify_with_id(42012, "FixItGarage · Wipers", &s.wiper_due_warning());
        any = true;
    }
    if s.has_low_tread() {
        notify_with_id(42013, "FixItGarage · Tread", &s.tread_warning());
        any = true;
    }
    if any {
        s.mark_notified();
    } else if s.has_due_reminders() {
        // Fallback summary
        notify("FixItGarage", &s.due_reminders_summary());
        s.mark_notified();
    }
}

/// Register AlarmManager wakes for open date-based reminders (Android).
/// Also writes `fig_alarms.json` so BootReceiver can re-register after reboot.
fn reschedule_reminder_alarms(s: &AppState) {
    // Cap concurrent alarms to avoid binder spam
    let alarms: Vec<(i32, i64, String)> = s.future_date_alarms().into_iter().take(24).collect();
    for (code, due, label) in &alarms {
        schedule_app_wake(*code, *due, label);
    }
    // Persist full list (may be empty) so boot does not keep stale alarms forever
    write_alarm_schedule(&alarms);
}

/// Android entry point (NativeActivity / android-activity).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Failed to init Slint Android backend");
    run_app().expect("FixItGarage UI failed");
}
