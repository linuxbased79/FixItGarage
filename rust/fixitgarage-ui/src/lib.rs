//! FixItGarage Slint UI — shared library for desktop binary and Android cdylib.

mod platform;
mod receipt_parse;
mod state;
mod units;
mod webdav;

use chrono::{TimeZone, Utc};
use platform::{
    cancel_app_wake, capture_issue_photo_path, notify, notify_with_id, open_ocr_helper, open_url,
    read_clipboard, schedule_app_wake, share_text, share_text_to_cloud, PKG_DROPBOX,
    PKG_GOOGLE_DRIVE, PKG_ONEDRIVE, PKG_PROTON_DRIVE,
};
use receipt_parse::{parse_receipt_text, parse_tire_receipt_text};
use state::{reminder_status_line_units, AppState};
use std::cell::RefCell;
use std::rc::Rc;
use units::{
    display_to_gallons, display_to_miles, display_to_mm, gallons_to_display, miles_to_display,
    mm_to_display, oil_level_options,
};

slint::include_modules!();

fn format_service_date(epoch_ms: i64) -> String {
    match Utc.timestamp_millis_opt(epoch_ms) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => String::new(),
    }
}

/// Wire Slint properties/callbacks to AppState and show the window.
pub fn run_app() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let state = Rc::new(RefCell::new(AppState::load()));

    refresh_ui(&ui, &state.borrow());
    // Due notifications (all vehicles) + schedule future date alarms
    {
        let mut s = state.borrow_mut();
        fire_due_notifications(&mut s);
        reschedule_reminder_alarms(&s);
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_finish_wizard(move |mode| {
            let mut s = state.borrow_mut();
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
            let s = state.borrow();
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_page(page);
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_select_vehicle(move |id| {
            let mut s = state.borrow_mut();
            s.select_vehicle(id as u64);
            if let Some(ui) = ui_weak.upgrade() {
                if let Some(v) = s.selected_vehicle() {
                    ui.set_form_name(v.name.clone().into());
                    ui.set_form_make(v.make.clone().into());
                    ui.set_form_model(v.model.clone().into());
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
                let mut s = state.borrow_mut();
                let name = ui.get_form_name().to_string();
                let make = ui.get_form_make().to_string();
                let model = ui.get_form_model().to_string();
                let year = ui.get_form_year().parse().ok();
                let u = s.unit_system();
                let mileage = display_to_miles(
                    ui.get_form_mileage().parse().unwrap_or(0),
                    u,
                );
                s.add_vehicle(name, make, model, year, mileage);
                ui.set_form_name("".into());
                ui.set_form_make("".into());
                ui.set_form_model("".into());
                ui.set_form_year("".into());
                ui.set_form_mileage("".into());
                ui.set_status_message("Vehicle saved (oil-level reminder in 3 months).".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_update_selected_mileage(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
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
                );
                ui.set_status_message("Vehicle details updated.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_selected_vehicle(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
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
            let mut s = state.borrow_mut();
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
            let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
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
        ui.on_parse_tire_receipt(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let paste = ui.get_tire_rcp_paste().to_string();
                if paste.trim().is_empty() {
                    ui.set_status_message("Paste tire receipt text first.".into());
                    return;
                }
                let p = parse_tire_receipt_text(&paste);
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
                    ui.set_tire_buy_mileage(mi.to_string().into());
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
                let mut s = state.borrow_mut();
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
                );
                ui.set_status_message("Distance per tire saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_add_service(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
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
                );
                ui.set_svc_title("".into());
                ui.set_svc_mileage("".into());
                ui.set_svc_cost("".into());
                ui.set_svc_labor("".into());
                ui.set_svc_gallons("".into());
                ui.set_svc_fuel("".into());
                ui.set_svc_shop("".into());
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
                refresh_ui(&ui, &state.borrow());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_pattern(move |pattern| {
            let mut s = state.borrow_mut();
            s.tire_pattern = pattern.to_string();
            s.save();
            if let Some(ui) = ui_weak.upgrade() {
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_apply_rotation(move || {
            let mut s = state.borrow_mut();
            s.apply_tire_rotation();
            s.save();
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_message("Rotation applied.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_user_mode(move |mode| {
            let mut s = state.borrow_mut();
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
            let mut s = state.borrow_mut();
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
        ui.on_set_dark_mode(move |mode| {
            let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
                let u = s.unit_system();
                let mi = ui
                    .get_part_mileage()
                    .parse()
                    .ok()
                    .map(|v| display_to_miles(v, u));
                s.upsert_part(
                    ui.get_part_type().to_string(),
                    ui.get_part_brand().to_string(),
                    ui.get_part_number().to_string(),
                    ui.get_part_oil().to_string(),
                    ui.get_part_notes().to_string(),
                    mi,
                );
                ui.set_status_message("Part saved.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_component(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
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
                ui.set_oil_level_choice(choice);
                let _ = state;
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
                let mut s = state.borrow_mut();
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
                let mut s = state.borrow_mut();
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
        ui.on_save_tire_purchase(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
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
            let s = state.borrow();
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
            let s = state.borrow();
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
            let s = state.borrow();
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
                        *state.borrow_mut() = new_state;
                        ui.set_status_message("Backup restored.".into());
                        refresh_ui(&ui, &state.borrow());
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
            let s = state.borrow();
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
            let s = state.borrow();
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
                ui.set_status_message("Camera opened for receipt (paste OCR text to auto-fill).".into());
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
                let u = state.borrow().unit_system();
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
        ui.on_paste_receipt_clipboard(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match read_clipboard() {
                    Some(text) => {
                        ui.set_rcp_paste(text.into());
                        ui.set_status_message(
                            "Clipboard pasted — tap Auto-fill from pasted text.".into(),
                        );
                    }
                    None => ui.set_status_message("Clipboard empty or unavailable.".into()),
                }
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_paste_tire_clipboard(move || {
            if let Some(ui) = ui_weak.upgrade() {
                match read_clipboard() {
                    Some(text) => {
                        ui.set_tire_rcp_paste(text.into());
                        ui.set_status_message(
                            "Clipboard pasted — tap Parse receipt text.".into(),
                        );
                    }
                    None => ui.set_status_message("Clipboard empty or unavailable.".into()),
                }
            }
        });
    }

    {
        ui.on_open_ocr_helper(move || {
            open_ocr_helper();
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_cloud_settings(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
                s.set_cloud_settings(
                    ui.get_cloud_url().to_string(),
                    ui.get_cloud_user().to_string(),
                    ui.get_cloud_pass().to_string(),
                );
                ui.set_status_message("Cloud settings saved on device.".into());
                refresh_ui(&ui, &s);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_upload_cloud_backup(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let s = state.borrow();
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
                refresh_ui(&ui, &state.borrow());
            }
        });
    }

    ui.on_open_feedback(|| {
        open_url("https://github.com/linuxbased79/FixItGarage/issues");
    });
    ui.on_open_donate(|| {
        open_url("https://github.com/linuxbased79/FixItGarage#donate");
    });

    ui.run()
}

fn refresh_ui(ui: &MainWindow, state: &AppState) {
    ui.set_wizard_done(state.wizard_done);
    ui.set_user_mode(state.user_mode.clone().into());
    let dark = state.dark_mode.eq_ignore_ascii_case("DARK");
    ui.set_dark_mode(if dark { "DARK" } else { "LIGHT" }.into());
    Theme::get(ui).set_dark(dark);

    let flags = state.feature_flags();
    ui.set_show_tires(flags.show_tires);
    ui.set_show_parts(flags.show_parts);
    ui.set_show_diy_trackers(flags.show_diy_trackers);
    ui.set_show_shop_emphasis(flags.show_shop_emphasis);

    ui.set_mode_label(state.mode_label().into());
    ui.set_vehicle_count_label(format!("{} vehicles", state.vehicles.len()).into());

    let sel_name = state
        .selected_vehicle()
        .map(|v| v.name.clone())
        .unwrap_or_else(|| "No vehicle selected".into());
    ui.set_selected_vehicle_label(sel_name.into());
    ui.set_selected_vehicle_id(state.selected_vehicle_id.unwrap_or(0) as i32);

    let u = state.unit_system();
    ui.set_units(u.as_str().into());
    ui.set_label_distance(u.distance_label().into());
    ui.set_label_fuel(u.fuel_label().into());
    ui.set_label_economy(u.economy_unit().into());
    ui.set_label_tread(u.tread_unit().into());
    ui.set_unit_distance(u.distance_unit().into());
    ui.set_unit_fuel(u.fuel_unit().into());
    // Oil level choices for current unit system
    let oil_opts = oil_level_options(u);
    ui.set_oil_opt_0(oil_opts[0].into());
    ui.set_oil_opt_1(oil_opts[1].into());
    ui.set_oil_opt_2(oil_opts[2].into());
    ui.set_oil_opt_3(oil_opts[3].into());
    ui.set_oil_opt_4(oil_opts[4].into());
    ui.set_oil_opt_5(oil_opts[5].into());

    ui.set_mpg_label(state.mpg_label().into());
    ui.set_tire_fl(state.tire_layout.fl.clone().into());
    ui.set_tire_fr(state.tire_layout.fr.clone().into());
    ui.set_tire_rl(state.tire_layout.rl.clone().into());
    ui.set_tire_rr(state.tire_layout.rr.clone().into());
    let after = state.preview_after_layout();
    ui.set_tire_after_fl(after.fl.into());
    ui.set_tire_after_fr(after.fr.into());
    ui.set_tire_after_rl(after.rl.into());
    ui.set_tire_after_rr(after.rr.into());
    ui.set_tire_pattern(state.tire_pattern.clone().into());
    ui.set_tire_preview(state.tire_preview().into());

    // Tracker summaries
    ui.set_part_air(state.part_summary("ENGINE_AIR_FILTER").into());
    ui.set_part_cabin(state.part_summary("CABIN_FILTER").into());
    ui.set_part_oil_filter(state.part_summary("OIL_FILTER").into());
    ui.set_part_oil_type(state.part_summary("OIL_TYPE").into());
    ui.set_sum_battery(state.component_summary("BATTERY").into());
    // LHD convention: driver = left, passenger = right (most US / GrapheneOS devices)
    ui.set_sum_wiper_driver(state.component_summary("WIPER_DRIVER").into());
    ui.set_sum_wiper_passenger(state.component_summary("WIPER_PASSENGER").into());
    ui.set_sum_wiper_rear(state.component_summary("WIPER_REAR").into());
    ui.set_sum_brake_f(state.component_summary("BRAKE_PADS_FRONT").into());
    ui.set_sum_brake_r(state.component_summary("BRAKE_PADS_REAR").into());
    ui.set_sum_brake_fluid(state.component_summary("BRAKE_FLUID").into());

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
                subtitle: format!(
                    "{} · {} · {}",
                    format_service_date(s.date_epoch_ms),
                    units::format_distance(s.mileage, u),
                    s.source.as_str()
                )
                .into(),
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
                title: r.title.clone().into(),
                detail: reminder_status_line_units(r, mileage, u).into(),
                is_oil_level: is_oil,
            }
        })
        .collect();
    ui.set_reminders(slint::ModelRc::new(slint::VecModel::from(reminders)));
    ui.set_last_oil_level_summary(state.last_oil_level_summary().into());
    // Keep a sensible default if empty
    if ui.get_oil_level_choice().is_empty() {
        ui.set_oil_level_choice("Full".into());
    }

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
    ui.set_cloud_url(state.cloud_webdav_url.clone().into());
    ui.set_cloud_user(state.cloud_username.clone().into());
    // Do not push password into UI on every refresh if user is typing — only seed when empty
    if ui.get_cloud_pass().is_empty() && !state.cloud_password.is_empty() {
        ui.set_cloud_pass(state.cloud_password.clone().into());
    }
    let t = state.tread_for_selected();
    let fmt_t = |v: f64| {
        let d = mm_to_display(v, u);
        if u.is_metric() {
            format!("{d:.1}")
        } else {
            format!("{d:.1}")
        }
    };
    ui.set_tread_fl(t.fl.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_fr(t.fr.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_rl(t.rl.map(fmt_t).unwrap_or_default().into());
    ui.set_tread_rr(t.rr.map(fmt_t).unwrap_or_default().into());
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
fn reschedule_reminder_alarms(s: &AppState) {
    // Cap concurrent alarms to avoid binder spam
    for (code, due, label) in s.future_date_alarms().into_iter().take(24) {
        schedule_app_wake(code, due, &label);
    }
}

/// Android entry point (NativeActivity / android-activity).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Failed to init Slint Android backend");
    run_app().expect("FixItGarage UI failed");
}
