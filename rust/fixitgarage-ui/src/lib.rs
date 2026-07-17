//! FixItGarage Slint UI — shared library for desktop binary and Android cdylib.

mod platform;
mod state;

use chrono::{TimeZone, Utc};
use platform::open_url;
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
        ui.on_add_service(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut s = state.borrow_mut();
                if s.vehicles.is_empty() {
                    ui.set_status_message("Add a vehicle first.".into());
                    return;
                }
                let title = ui.get_svc_title().to_string();
                let mileage = ui.get_svc_mileage().parse().unwrap_or(0);
                let cost = ui.get_svc_cost().parse().unwrap_or(0.0);
                let gallons = {
                    let g = ui.get_svc_gallons().to_string();
                    if g.trim().is_empty() {
                        None
                    } else {
                        g.parse().ok()
                    }
                };
                let source = ui.get_svc_source().to_string();
                s.add_service(title, mileage, &source, cost, gallons);
                ui.set_svc_title("".into());
                ui.set_svc_mileage("".into());
                ui.set_svc_cost("".into());
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
                // no full refresh needed
                let _ = state;
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
    ui.set_sum_wiper_f(state.component_summary("WIPER_FRONT").into());
    ui.set_sum_wiper_r(state.component_summary("WIPER_REAR").into());
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
}

/// Android entry point (NativeActivity / android-activity).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Failed to init Slint Android backend");
    run_app().expect("FixItGarage UI failed");
}
