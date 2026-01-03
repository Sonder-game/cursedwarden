# Cursed Warden (Проклятый Страж) - Development Guide

## Overview
**The Cursed Warden** is a Narrative Roguelite / Auto-Battler with an "Inventory Tetris" mechanic, built using the **Bevy Game Engine** (Rust).

## Documentation
* **GDD (Game Design Document):** Located in `docs/` folder. Please refer to this for all gameplay logic, formulas, and architectural decisions.

## Technical Stack
* **Language:** Rust
* **Engine:** Bevy (latest stable)
* **Architecture:** ECS (Entity Component System), Plugin-based.

## Setup Instructions

### 1. Prerequisites

#### Windows (User Environment)
1.  **Install Rust:** Download and install `rustup` from [rust-lang.org](https://www.rust-lang.org/tools/install).
2.  **Install C++ Build Tools:** Install "Desktop development with C++" via Visual Studio Installer (required for Rust linker).
3.  **Optimization (Optional but Recommended):** Install the `lld` linker for faster builds.
    *   `cargo install -f cargo-binutils`
    *   `rustup component add llvm-tools-preview`

#### Linux (Dev/Sandbox Environment)
Bevy requires specific system libraries to interact with the OS (Windowing, Audio, Input).
Ensure the following are installed:
```bash
# Ubuntu/Debian based
sudo apt-get update && sudo apt-get install -y \
    g++ pkg-config libx11-dev libasound2-dev libudev-dev \
    libwayland-dev libxkbcommon-dev
```
*Note: In the sandbox, if `sudo` is unavailable, rely on pre-installed tools or request assistance if builds fail due to missing sys-libs.*

### 2. Running the Project

**Development Mode (Fast Compile):**
We use dynamic linking for development to speed up incremental builds.
```bash
# Run the game
cargo run

# Run with dynamic linking explicitly (if configured in Cargo.toml features)
cargo run --features bevy/dynamic_linking
```

**Release Mode (Optimized):**
```bash
cargo run --release
```

## Project Structure
The project follows a modular plugin architecture as described in the GDD.

```
src/
├── main.rs            # Entry point, App initialization
├── lib.rs             # Lib root, plugin registration
├── core/              # Core systems (AssetLoading, GameState)
├── inventory/         # Grid logic, Tetris mechanics
├── combat/            # Auto-battler simulation
├── meta/              # Time, Economy, Reputation
├── narrative/         # Event system
└── ui/                # UI layout and styling
```

## Development Rules
1.  **Modular Design:** Every major system should be a Bevy `Plugin`.
2.  **ECS First:** Separate Data (Components) from Logic (Systems).
3.  **State Management:** Use `States` to isolate game phases (Day, Evening/Inventory, Night/Combat).
4.  **No Artifacts:** Do not edit files in `target/`.
5.  **Tests:** Write unit tests for complex logic (e.g., inventory overlap checks, damage formulas).

## Troubleshooting
* **"Linking with `cc` failed":** Missing C++ compiler or system libraries. Check Prerequisites.
* **Slow Compile Times:** Ensure you are using the dynamic linking feature during dev.
