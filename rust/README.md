# FixItGarage — Rust workspace

Pure **Rust** product path for FixItGarage (GPL-3.0): domain logic, CLI, and a **Slint mobile/desktop UI**.

## Crates

| Crate | Role |
|-------|------|
| `fixitgarage-core` | MPG, tire rotation, CSV export, cost rollups, reminders |
| `fixitgarage-cli` | `fixitgarage` binary for local tools |
| **`fixitgarage-ui`** | **Full mobile/desktop UI (Slint)** — primary Rust app |

## Build & test

```bash
cd rust
cargo build --release
cargo test
cargo build -p fixitgarage-ui --release
```

## Run the GUI (desktop)

```bash
cargo run -p fixitgarage-ui --release
```

Phone-sized window with wizard, vehicles, services, tire diagram, costs, settings.  
Android APK steps: [fixitgarage-ui/README.md](fixitgarage-ui/README.md).

## CLI examples

```bash
# Average MPG from fill-ups (odometer:gallons)
cargo run -p fixitgarage-cli -- mpg 10000:10 10300:10 10580:10

# Tire rotation preview
cargo run -p fixitgarage-cli -- rotate forward_cross --fl A --fr B --rl C --rr D

# Sample costs + CSV
cargo run -p fixitgarage-cli -- demo
```

## Why Rust + Slint?

- Single language for domain logic **and** UI  
- Official Slint Android backend (GrapheneOS-friendly native binary)  
- No Google Play Services  
- GPL-3.0 aligned FOSS licensing  

The Kotlin Compose project under `../app` remains as an optional/reference shell.
