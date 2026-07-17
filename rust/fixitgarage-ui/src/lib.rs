//! FixItGarage Slint UI — shared library for desktop binary and Android cdylib.

mod state;

use chrono::{TimeZone, Utc};
use state::AppState;
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn open_url(url: &str) {
    // Best-effort: xdg-open on Linux, ignored if unavailable (e.g. pure Android later).
    #[cfg(target_os = "android")]
    {
        let _ = url;
        // Android URL open can be wired via JNI later.
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

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
                ui.set_status_message("Vehicle saved.".into());
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
        ui.on_export_csv(move || {
            let s = state.borrow();
            let csv = s.export_csv();
            // Also write next to data dir
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
    // Keep global Theme in sync (also driven by Slint changed handler)
    Theme::get(ui).set_dark(dark);
    ui.set_mode_label(state.mode_label().into());
    ui.set_vehicle_count_label(format!("{} vehicles", state.vehicles.len()).into());
    ui.set_mpg_label(state.mpg_label().into());
    ui.set_tire_fl(state.tire_layout.fl.clone().into());
    ui.set_tire_fr(state.tire_layout.fr.clone().into());
    ui.set_tire_rl(state.tire_layout.rl.clone().into());
    ui.set_tire_rr(state.tire_layout.rr.clone().into());
    ui.set_tire_pattern(state.tire_pattern.clone().into());
    ui.set_tire_preview(state.tire_preview().into());

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
        ui.set_last_service_detail("Scan a receipt or add a service.".into());
    }

    let vehicles: Vec<VehicleRow> = state
        .vehicles
        .iter()
        .map(|v| {
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
                name: v.name.clone().into(),
                subtitle: subtitle.into(),
                mileage: format!("{} mi", v.current_mileage).into(),
            }
        })
        .collect();
    ui.set_vehicles(slint::ModelRc::new(slint::VecModel::from(vehicles)));

    let mut services: Vec<_> = state.services.clone();
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
}

/// Android entry point (NativeActivity / android-activity).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Failed to init Slint Android backend");
    run_app().expect("FixItGarage UI failed");
}
