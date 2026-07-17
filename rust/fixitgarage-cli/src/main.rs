//! FixItGarage CLI — pure Rust helpers (GPL-3.0).

use chrono::Utc;
use clap::{Parser, Subcommand};
use fixitgarage_core::models::{ServiceRecord, ServiceSource};
use fixitgarage_core::{
    apply_rotation, average_mpg, services_to_csv, summarize_costs, RotationPattern, TireLayout,
};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(
    name = "fixitgarage",
    about = "FixItGarage Rust tools — MPG, tire rotation, CSV, costs",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compute average MPG from fill-ups: mileage:gallons pairs
    Mpg {
        /// e.g. 10000:10.2 10300:9.8
        fills: Vec<String>,
    },
    /// Preview a tire rotation pattern on layout A B C D (FL FR RL RR)
    Rotate {
        /// forward_cross | rearward_cross | x_pattern | side_to_side
        pattern: String,
        #[arg(long, default_value = "A")]
        fl: String,
        #[arg(long, default_value = "B")]
        fr: String,
        #[arg(long, default_value = "C")]
        rl: String,
        #[arg(long, default_value = "D")]
        rr: String,
    },
    /// Demo cost rollup and sample CSV export
    Demo {
        /// Optional path to write sample CSV
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Mpg { fills } => {
            let parsed: Result<Vec<(u32, f64)>, String> = fills
                .iter()
                .map(|s| {
                    let (m, g) = s
                        .split_once(':')
                        .ok_or_else(|| format!("expected mileage:gallons, got {s}"))?;
                    let mileage: u32 = m
                        .parse()
                        .map_err(|_| format!("bad mileage in {s}"))?;
                    let gallons: f64 = g
                        .parse()
                        .map_err(|_| format!("bad gallons in {s}"))?;
                    Ok((mileage, gallons))
                })
                .collect();
            match parsed {
                Ok(fills) => match average_mpg(&fills) {
                    Some(mpg) => println!("{mpg:.2} MPG average"),
                    None => {
                        eprintln!("Need at least two valid consecutive fill-ups");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Rotate {
            pattern,
            fl,
            fr,
            rl,
            rr,
        } => {
            let pattern = match RotationPattern::from_str(&pattern) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };
            let before = TireLayout {
                fl,
                fr,
                rl,
                rr,
                spare: String::new(),
            };
            let after = apply_rotation(&before, pattern, false);
            println!("Pattern: {}", pattern.label());
            println!("Before:\n{before}");
            println!("After:\n{after}");
        }
        Commands::Demo { out } => {
            let now = Utc::now().timestamp_millis();
            let sample = vec![
                ServiceRecord {
                    id: 1,
                    vehicle_id: 1,
                    date_epoch_ms: now,
                    mileage: 42000,
                    title: "Oil change".into(),
                    source: ServiceSource::Diy,
                    labor_cost: 0.0,
                    parts_cost: 48.0,
                    gallons: None,
                    fuel_cost: None,
                    shop_name: String::new(),
                },
                ServiceRecord {
                    id: 2,
                    vehicle_id: 1,
                    date_epoch_ms: now,
                    mileage: 42100,
                    title: "Fuel".into(),
                    source: ServiceSource::Diy,
                    labor_cost: 0.0,
                    parts_cost: 0.0,
                    gallons: Some(12.5),
                    fuel_cost: Some(45.0),
                    shop_name: String::new(),
                },
            ];
            let costs = summarize_costs(&sample, now);
            println!(
                "Costs — month: ${:.2}  year: ${:.2}  all-time: ${:.2}",
                costs.month_total, costs.year_total, costs.all_time_total
            );
            let csv = services_to_csv(&sample);
            if let Some(path) = out {
                std::fs::write(&path, &csv).expect("write csv");
                println!("Wrote {}", path.display());
            } else {
                print!("{csv}");
            }
        }
    }
}
