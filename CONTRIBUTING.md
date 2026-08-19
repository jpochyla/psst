# Contributing to Psst

Thank you for your interest in contributing to Psst! This guide will help you get started.

## Project Structure

Psst is organized as a Cargo workspace with three crates:

- **psst-core** — Core audio playback, Spotify session management, and CDN access
- **psst-gui** — Desktop GUI built with Druid
- **psst-cli** — Command-line interface

## Platform-Specific Code

Psst runs on Linux, macOS, and Windows. Platform-specific code lives in:

- **Linux**: Audio output via `cpal`, packaging scripts in `snap/`, desktop integration files in `.desktop` and `metainfo/`
- **macOS**: Audio output via `cpal`, app bundling handled in build configuration
- **Windows**: Audio output via `cpal`, installer and packaging configuration

We are actively looking for **Linux and Windows platform maintainers** to help with:

- Packaging and distribution
- Platform-specific bug triage
- CI/CD pipeline maintenance for each platform
- Testing on target platforms

## Getting Started

1. Fork the repository and clone your fork
2. Install Rust via [rustup](https://rustup.rs/)
3. Install platform dependencies:
   - **Linux**: `libssl-dev`, `libasound2-dev`, `libgtk-3-dev`
   - **macOS**: Xcode command line tools
   - **Windows**: Visual Studio Build Tools with C++ workload
4. Build the project: `cargo build`
5. Run the GUI: `cargo run --bin psst-gui`

## Development Workflow

1. Create a branch from `main` for your change
2. Make focused, minimal changes that address a single concern
3. Ensure `cargo build` succeeds on your platform
4. Run `cargo fmt` to format code (see `.rustfmt.toml` for settings)
5. Run `cargo clippy` and address any warnings
6. Open a pull request with a clear description of the change

## Code Style

- Follow existing patterns in the codebase
- Use `cargo fmt` with the project's `.rustfmt.toml`
- Keep imports organized per `imports_granularity = "Crate"`

## Reporting Issues

- Search existing issues before opening a new one
- Include your OS, Rust version, and steps to reproduce
- For platform-specific bugs, tag the issue with the relevant platform

## Becoming a Platform Maintainer

If you are interested in helping maintain Psst on Linux or Windows, please open an issue expressing your interest. Platform maintainers help with:

- Reproducing and triaging platform-specific bugs
- Maintaining packaging and distribution for their platform
- Reviewing platform-related pull requests
- Ensuring CI passes on their platform
