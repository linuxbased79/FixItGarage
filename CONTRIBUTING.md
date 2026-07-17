# Contributing to FixItGarage

Thank you for helping build free vehicle-maintenance software.

## License

Contributions are accepted under the **GNU GPL v3.0** (or later, if the project is dual-licensed later — currently GPL-3.0 only).

## Development

1. Fork and clone the repo  
2. Open in Android Studio  
3. Use JDK 17  
4. `./gradlew :app:assembleDebug`  
5. Prefer small, focused pull requests  

## Principles

- Stay **GrapheneOS-friendly**: no hard dependency on Google Play Services for core features  
- Keep **F-Droid** builds free of proprietary libraries  
- Prefer local-first design; cloud features must be optional  
- Write or update unit tests for pure logic (e.g. MPG, CSV, rotation math)

## Code style

Kotlin, Jetpack Compose, Material 3. Match existing package layout under `org.fixitgarage.app`.
