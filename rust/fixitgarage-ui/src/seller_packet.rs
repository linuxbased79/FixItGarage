//! “Selling my car” — professional maintenance packet for the selected vehicle.
//! Produces a PDF (printpdf) plus a plain-text summary for share sheets.

use crate::state::AppState;
use crate::units::{format_distance, UnitSystem};
use chrono::{TimeZone, Utc};
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

fn fmt_date(epoch_ms: i64) -> String {
    match Utc.timestamp_millis_opt(epoch_ms) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => "—".into(),
    }
}

fn money(v: f64) -> String {
    format!("${v:.2}")
}

/// Build seller packet for the currently selected vehicle.
/// Returns `(pdf_path, text_summary)`.
pub fn build_seller_packet(state: &AppState) -> Result<(PathBuf, String), String> {
    let v = state
        .selected_vehicle()
        .ok_or_else(|| "Select a vehicle first (Cars tab).".to_string())?;
    let units = state.unit_system();
    let text = build_text_summary(state, units);
    let pdf_path = write_pdf(state, units, &text)?;
    Ok((pdf_path, text))
}

fn build_text_summary(state: &AppState, units: UnitSystem) -> String {
    let v = match state.selected_vehicle() {
        Some(v) => v,
        None => return String::new(),
    };
    let mut out = String::new();
    out.push_str("══════════════════════════════════════\n");
    out.push_str("  FIXITGARAGE — SELLER MAINTENANCE PACKET\n");
    out.push_str("══════════════════════════════════════\n\n");
    out.push_str(&format!("Generated: {}\n", Utc::now().format("%Y-%m-%d %H:%M UTC")));
    out.push_str("Purpose: Document service history for a prospective buyer.\n");
    out.push_str("Source: FixItGarage local log (seller-provided).\n\n");

    out.push_str("── VEHICLE ──\n");
    out.push_str(&format!("Name:     {}\n", v.name));
    out.push_str(&format!(
        "Vehicle:  {} {} {}\n",
        v.year.map(|y| y.to_string()).unwrap_or_else(|| "—".into()),
        v.make,
        v.model
    ));
    if !v.vin.is_empty() {
        out.push_str(&format!("VIN:      {}\n", v.vin));
    }
    out.push_str(&format!(
        "Odometer: {}\n\n",
        format_distance(v.current_mileage, units)
    ));

    let services = state.services_for_selected();
    let mut total = 0.0f64;
    out.push_str(&format!("── SERVICE HISTORY ({} records) ──\n", services.len()));
    if services.is_empty() {
        out.push_str("(No service records logged.)\n");
    } else {
        for r in &services {
            total += r.total_cost();
            out.push_str(&format!(
                "• {} | {} mi | {} | {}\n  {} | parts {} labor {}{}\n",
                fmt_date(r.date_epoch_ms),
                r.mileage,
                r.source.as_str(),
                r.title,
                money(r.total_cost()),
                money(r.parts_cost),
                money(r.labor_cost),
                if r.shop_name.is_empty() {
                    String::new()
                } else {
                    format!(" | shop: {}", r.shop_name)
                }
            ));
            if !r.notes.is_empty() {
                out.push_str(&format!("  Notes: {}\n", r.notes));
            }
        }
    }
    out.push_str(&format!("\nService total logged: {}\n\n", money(total)));

    // Tires
    let tires: Vec<_> = state
        .tire_purchases
        .iter()
        .filter(|t| t.vehicle_id == v.id)
        .collect();
    out.push_str(&format!("── TIRE PURCHASES ({}) ──\n", tires.len()));
    let mut tire_total = 0.0;
    for t in &tires {
        tire_total += t.cost;
        out.push_str(&format!(
            "• {} | {} {} {} | {}{}\n",
            fmt_date(t.date_epoch_ms),
            t.brand,
            t.model,
            t.size,
            money(t.cost),
            t.mileage
                .map(|m| format!(" @ {}", format_distance(m, units)))
                .unwrap_or_default()
        ));
    }
    if tires.is_empty() {
        out.push_str("(None logged.)\n");
    }
    out.push_str(&format!("Tire purchase total: {}\n\n", money(tire_total)));

    let rots: Vec<_> = state
        .tire_rotations
        .iter()
        .filter(|r| r.vehicle_id == v.id)
        .collect();
    out.push_str(&format!("── TIRE ROTATIONS ({}) ──\n", rots.len()));
    for r in &rots {
        out.push_str(&format!(
            "• {} | pattern {}{}\n",
            fmt_date(r.date_epoch_ms),
            r.pattern,
            r.mileage
                .map(|m| format!(" @ {}", format_distance(m, units)))
                .unwrap_or_default()
        ));
    }
    if rots.is_empty() {
        out.push_str("(None logged.)\n");
    }
    out.push('\n');

    // Parts
    let parts: Vec<_> = state.parts.iter().filter(|p| p.vehicle_id == v.id).collect();
    out.push_str(&format!("── PARTS LOG ({}) ──\n", parts.len()));
    for p in &parts {
        out.push_str(&format!(
            "• {} | {} {} | #{}{}\n",
            p.part_type,
            p.brand,
            p.oil_viscosity,
            p.part_number,
            if p.notes.is_empty() {
                String::new()
            } else {
                format!(" — {}", p.notes)
            }
        ));
    }
    if parts.is_empty() {
        out.push_str("(None logged.)\n");
    }
    out.push('\n');

    // Components
    let comps: Vec<_> = state
        .components
        .iter()
        .filter(|c| c.vehicle_id == v.id)
        .collect();
    out.push_str(&format!("── COMPONENTS ({}) ──\n", comps.len()));
    for c in &comps {
        let installed = c
            .installed_epoch_ms
            .map(fmt_date)
            .unwrap_or_else(|| "—".into());
        out.push_str(&format!(
            "• {} | installed {}{}\n",
            c.component_type,
            installed,
            if c.notes.is_empty() {
                String::new()
            } else {
                format!(" — {}", c.notes)
            }
        ));
    }
    if comps.is_empty() {
        out.push_str("(None logged.)\n");
    }
    out.push('\n');

    // Photos
    let photos: Vec<_> = state
        .issue_photos
        .iter()
        .filter(|p| p.vehicle_id == v.id)
        .collect();
    out.push_str(&format!("── ISSUE / DOCUMENT PHOTOS ({}) ──\n", photos.len()));
    for p in &photos {
        out.push_str(&format!(
            "• {} | {}{}\n  Path: {}\n",
            fmt_date(p.created_epoch_ms),
            p.caption,
            if p.notes.is_empty() {
                String::new()
            } else {
                format!(" — {}", p.notes)
            },
            if p.file_path.is_empty() {
                "(no file path)"
            } else {
                p.file_path.as_str()
            }
        ));
    }
    if photos.is_empty() {
        out.push_str("(None logged. Attach receipts/issues in the app for a stronger packet.)\n");
    }

    out.push_str("\n── TOTALS ──\n");
    out.push_str(&format!("Services:  {}\n", money(total)));
    out.push_str(&format!("Tires:     {}\n", money(tire_total)));
    out.push_str(&format!(
        "Combined:  {}\n",
        money(total + tire_total)
    ));

    out.push_str("\n── DISCLAIMER ──\n");
    out.push_str(
        "This packet is generated from the seller’s FixItGarage log. It is not a certified\n\
         inspection or warranty. Buyers should verify records, open recalls (NHTSA), and\n\
         obtain an independent inspection. Free open-source app: fixitgarage (GPL-3).\n",
    );
    out
}

fn write_pdf(state: &AppState, units: UnitSystem, full_text: &str) -> Result<PathBuf, String> {
    let v = state
        .selected_vehicle()
        .ok_or_else(|| "No vehicle selected".to_string())?;

    let (doc, page1, layer1) = PdfDocument::new(
        "FixItGarage Seller Packet",
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("font: {e}"))?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("bold: {e}"))?;

    let mut pages: Vec<(PdfPageIndex, PdfLayerIndex)> = vec![(page1, layer1)];
    let mut y = 275.0f32;
    let left = 18.0f32;
    let line_h = 5.0f32;
    let bottom = 18.0f32;

    let new_page = |doc: &PdfDocumentReference, pages: &mut Vec<(PdfPageIndex, PdfLayerIndex)>| {
        let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer");
        pages.push((p, l));
    };

    let write_line = |doc: &PdfDocumentReference,
                      pages: &mut Vec<(PdfPageIndex, PdfLayerIndex)>,
                      y: &mut f32,
                      text: &str,
                      size: f32,
                      use_bold: bool| {
        if *y < bottom + line_h {
            new_page(doc, pages);
            *y = 275.0;
        }
        let (page, layer) = *pages.last().unwrap();
        let layer = doc.get_page(page).get_layer(layer);
        let f = if use_bold { &bold } else { &font };
        // printpdf panics on some control chars — sanitize
        let clean: String = text
            .chars()
            .map(|c| if c.is_control() && c != '\t' { ' ' } else { c })
            .collect();
        // Helvetica is WinAnsi — strip non-latin1-ish for safety
        let clean: String = clean
            .chars()
            .map(|c| if (c as u32) < 256 { c } else { '?' })
            .collect();
        let max_chars = 95usize;
        if clean.chars().count() <= max_chars {
            layer.use_text(&clean, size, Mm(left), Mm(*y), f);
            *y -= line_h * if size > 12.0 { 1.4 } else { 1.0 };
        } else {
            let mut rest = clean.as_str();
            while !rest.is_empty() {
                if *y < bottom + line_h {
                    new_page(doc, pages);
                    *y = 275.0;
                }
                let (page, layer) = *pages.last().unwrap();
                let layer = doc.get_page(page).get_layer(layer);
                let take = rest
                    .char_indices()
                    .nth(max_chars)
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
                let (chunk, next) = rest.split_at(take);
                layer.use_text(chunk, size, Mm(left), Mm(*y), f);
                *y -= line_h;
                rest = next;
            }
        }
    };

    write_line(
        &doc,
        &mut pages,
        &mut y,
        "FIXITGARAGE — SELLER MAINTENANCE PACKET",
        16.0,
        true,
    );
    write_line(
        &doc,
        &mut pages,
        &mut y,
        &format!(
            "Generated {}  ·  For buyer review",
            Utc::now().format("%Y-%m-%d")
        ),
        10.0,
        false,
    );
    y -= 3.0;
    write_line(&doc, &mut pages, &mut y, "VEHICLE", 13.0, true);
    write_line(
        &doc,
        &mut pages,
        &mut y,
        &format!(
            "{} — {} {} {}  ·  {}",
            v.name,
            v.year.map(|y| y.to_string()).unwrap_or_else(|| "—".into()),
            v.make,
            v.model,
            format_distance(v.current_mileage, units)
        ),
        11.0,
        false,
    );
    if !v.vin.is_empty() {
        write_line(
            &doc,
            &mut pages,
            &mut y,
            &format!("VIN: {}", v.vin),
            11.0,
            false,
        );
    }
    y -= 2.0;

    // Stream body lines from text summary (skip banner lines already written)
    for line in full_text.lines() {
        if line.starts_with('═') || line.contains("SELLER MAINTENANCE PACKET") {
            continue;
        }
        if line.starts_with("──") {
            y -= 2.0;
            write_line(&doc, &mut pages, &mut y, line.trim_matches('─').trim(), 12.0, true);
        } else if line.is_empty() {
            y -= 2.0;
        } else {
            write_line(&doc, &mut pages, &mut y, line, 9.5, false);
        }
    }

    let dir = packet_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let safe_name: String = v
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(32)
        .collect();
    let path = dir.join(format!(
        "seller-packet-{}-{}.pdf",
        safe_name,
        Utc::now().format("%Y%m%d")
    ));
    doc.save(&mut BufWriter::new(
        File::create(&path).map_err(|e| format!("create pdf: {e}"))?,
    ))
    .map_err(|e| format!("save pdf: {e}"))?;
    Ok(path)
}

fn packet_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fixitgarage")
        .join("seller-packets")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    #[test]
    fn builds_packet_for_vehicle() {
        let mut s = AppState::default();
        s.add_vehicle(
            "Daily".into(),
            "Honda".into(),
            "Accord".into(),
            Some(2003),
            120_000,
            "1HGCM82633A004352".into(),
        );
        s.add_service_full(
            "Oil change".into(),
            119_000,
            "DIY",
            45.0,
            0.0,
            None,
            None,
            1_700_000_000_000,
            String::new(),
            "Synthetic".into(),
        );
        let (path, text) = build_seller_packet(&s).expect("packet");
        assert!(path.is_file(), "{path:?}");
        assert!(text.contains("SELLER MAINTENANCE PACKET"));
        assert!(text.contains("Oil change"));
        assert!(text.contains("1HGCM82633A004352"));
        assert!(std::fs::metadata(&path).unwrap().len() > 500);
    }
}
