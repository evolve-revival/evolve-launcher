# Prerequisites

You require the rust toolchain (e.g. via rustup), Tauri (e.g. `cargo install tauri-cli`) and pnpm installed

# Setup

## Frontend

Run `pnpm i` to install all build-deps. You will be asked to allow install-scripts for esbuild. Accept the install-script.

## Tauri

You need to alter the build target dir, instructions are found under [src-tauri/.cargo/config.toml.template](./src-tauri/.cargo/config.toml.template)

# Developing

Run `cargo tauri dev`