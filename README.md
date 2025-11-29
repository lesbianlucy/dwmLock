# DwmLock

DwmLock is a Windows-only Rust application that captures the desktop, applies a blur, and presents a distraction-free lock UI that accepts a predefined password. It focuses on keeping the desktop hidden, suppressing disruptive key combos like `Ctrl+Alt+Delete`, and blocking toast notifications before the lock screen appears.

## Features
- Captures the current screen contents, applies a configurable blur, and renders a centered overlay with time/date and password prompt.
- Locks mouse movement to the center pixel and hides the cursor until the correct password is entered.
- Installs a low-level keyboard hook to swallow `Ctrl+Alt+Delete`.
- Optionally blanks selected monitors using overlays (configure in `%APPDATA%/DwmLock/dwmlock_settings.json` or the in-app dropdown for "keep on", "all", or "specific" monitors).
- Supports booting straight into the settings UI via the `--open-settings` flag or the "Open settings on startup" checkbox for quick tweaks before locking.
- Dismisses Windows toast/action center notifications on startup so they do not overlap the lock UI.

## Building
1. Install the latest stable Rust toolchain with the Windows MSVC target (`rustup target add x86_64-pc-windows-msvc`).
2. Ensure the Visual Studio Build Tools (with C++ support) are present so `link.exe` is available.
3. Clone this repository and run:
   ```bash
   cargo build --release --target x86_64-pc-windows-msvc
   ```
   The resulting executable is at `target/x86_64-pc-windows-msvc/release/dwmlock.exe`.

## Usage
1. Launch the binary on Windows; you will be prompted to confirm locking.
2. While locked, type the password (default `media`) and press Enter to release. Backspace erases characters; incorrect attempts trigger a warning state.
3. Update `%APPDATA%/DwmLock/dwmlock_settings.json` or run `dwmlock.exe --open-settings` to adjust the password, blur radius, startup behavior, and monitor blanking mode.

## Development Notes
- Run `cargo fmt --all` and `cargo clippy --all-targets --all-features` before sending PRs; CI enforces both.
- `cargo test --target x86_64-pc-windows-msvc` executes unit tests; run on both MSVC and GNU targets if you change platform-specific code.
- Core modules live under `src/` (`main.rs`, `render.rs`, `keyboard.rs`, `notifications.rs`, etc.); `AGENTS.md` contains contributor-oriented guidance.
