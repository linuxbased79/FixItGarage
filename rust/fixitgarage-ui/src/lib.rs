//! FixItGarage Slint UI — shared library for desktop binary and Android cdylib.

mod platform;
mod state;

use chrono::{TimeZone, Utc};
use platform::{capture_issue_photo_path, open_url};
use state::{reminder_status_line, AppState};
use std::cell::RefCell;
use std::rc::Rc;

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
                let mileage = ui.get_form_mileage().parse().unwrap_or(0);
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
                let mileage = ui.get_form_mileage().parse().unwrap_or(0);
                s.update_vehicle_mileage(id, mileage);
                ui.set_status_message(format!("Mileage updated to {mileage}.").into());
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
                let parse = |v: slint::SharedString| {
                    let t = v.to_string();
                    if t.trim().is_empty() {
                        None
                    } else {
                        t.parse().ok()
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
        let state = state.clone();
        ui.on_add_service(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
                if s.vehicles.is_empty() {
                    ui.set_status_message("Add a vehicle first.".into());
                    return;
                }
                let title = ui.get_svc_title().to_string();
                let mileage = ui.get_svc_mileage().parse().unwrap_or(0);
                let parts = ui.get_svc_cost().parse().unwrap_or(0.0);
                let labor = ui.get_svc_labor().parse().unwrap_or(0.0);
                let gallons = {
                    let g = ui.get_svc_gallons().to_string();
                    if g.trim().is_empty() {
                        None
                    } else {
                        g.parse().ok()
                    }
                };
                let source = ui.get_svc_source().to_string();
                s.add_service_full(
                    title,
                    mileage,
                    &source,
                    parts,
                    labor,
                    gallons,
                    None,
                    chrono::Utc::now().timestamp_millis(),
                    String::new(),
                );
                ui.set_svc_title("".into());
                ui.set_svc_mileage("".into());
                ui.set_svc_cost("".into());
                ui.set_svc_labor("".into());
                ui.set_svc_gallons("".into());
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
                s.upsert_part(
                    ui.get_part_type().to_string(),
                    ui.get_part_brand().to_string(),
                    ui.get_part_number().to_string(),
                    ui.get_part_oil().to_string(),
                    ui.get_part_notes().to_string(),
                    ui.get_part_mileage().parse().ok(),
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
                s.upsert_component(
                    ui.get_comp_type().to_string(),
                    ui.get_comp_notes().to_string(),
                    ui.get_comp_mileage().parse().ok(),
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
                s.add_reminder(
                    ui.get_rem_title().to_string(),
                    &ui.get_rem_date().to_string(),
                    ui.get_rem_mileage().parse().ok(),
                );
                ui.set_rem_title("".into());
                ui.set_rem_date("".into());
                ui.set_rem_mileage("".into());
                ui.set_status_message("Reminder saved.".into());
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
                s.complete_reminder(id as u64, Some(&oil));
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
                let gallons = {
                    let g = ui.get_rcp_gallons().to_string();
                    if g.trim().is_empty() {
                        None
                    } else {
                        g.parse().ok()
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
                    ui.get_rcp_mileage().parse().unwrap_or(0),
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
                s.add_tire_purchase(
                    ui.get_tire_brand().to_string(),
                    ui.get_tire_model().to_string(),
                    ui.get_tire_size().to_string(),
                    ui.get_tire_cost().parse().unwrap_or(0.0),
                    ui.get_tire_buy_mileage().parse().ok(),
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

    ui.set_mpg_label(state.mpg_label().into());
    ui.set_tire_fl(state.tire_layout.fl.clone().into());
    ui.set_tire_fr(state.tire_layout.fr.clone().into());
    ui.set_tire_rl(state.tire_layout.rl.clone().into());
    ui.set_tire_rr(state.tire_layout.rr.clone().into());
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
                "{} · {} mi · {}",
                format_service_date(last.date_epoch_ms),
                last.mileage,
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
                mileage: format!("{} mi", v.current_mileage).into(),
            }
        })
        .collect();
    ui.set_vehicles(slint::ModelRc::new(slint::VecModel::from(vehicles)));

    let mut services: Vec<_> = state
        .services_for_selected()
        .into_iter()
        .cloned()
        .collect();
    services.sort_by(|a, b| b.date_epoch_ms.cmp(&a.date_epoch_ms).then(b.id.cmp(&a.id)));
    let services: Vec<ServiceRow> = services
        .into_iter()
        .map(|s| {
            let total = s.total_cost();
            ServiceRow {
                id: s.id as i32,
                title: s.title.into(),
                subtitle: format!(
                    "{} · {} mi · {}",
                    format_service_date(s.date_epoch_ms),
                    s.mileage,
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
                detail: reminder_status_line(r, mileage).into(),
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
                .map(|m| format!("{m} mi"))
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
                .map(|m| format!("{m} mi"))
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
    ui.set_tread_summary(state.tread_summary().into());
    let t = state.tread_for_selected();
    ui.set_tread_fl(t.fl.map(|v| format!("{v}")).unwrap_or_default().into());
    ui.set_tread_fr(t.fr.map(|v| format!("{v}")).unwrap_or_default().into());
    ui.set_tread_rl(t.rl.map(|v| format!("{v}")).unwrap_or_default().into());
    ui.set_tread_rr(t.rr.map(|v| format!("{v}")).unwrap_or_default().into());
}

/// Android entry point (NativeActivity / android-activity).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Failed to init Slint Android backend");
    run_app().expect("FixItGarage UI failed");
}
