# DwmLock Architecture

## Module responsibilities

- `src/main.rs` is the orchestrator: it reads settings, optionally opens the UI, prepares the screen capture, initializes the shared `AppState`, installs the keyboard hook, and hands execution to the window loop from `src/ui/window.rs`.
- `src/ui/window.rs` owns the Win32 window class, message loop, painting, overlay lifecycle, blur rendering, and lock confirmation prompt. Moving this code into `src/ui` keeps the UI plumbing separate from the rest of the crate.
- `src/ui/settings_dialog.rs` renders the modal settings dialog with grouped controls for password, blur, monitor blanking, and the new options such as per-monitor text hints. Keeping it next to `window.rs` makes it easier to evolve the UI without cluttering the runtime logic.
- `src/settings.rs` serializes/deserializes user preferences (`Settings`) and returns defaults; `settings_dialog` consumes and mutates that struct before the main process saves it back with `persist_settings`.
- `src/monitors.rs` enumerates and blanks external displays. Blank overlays now optionally draw helper text so users know the lock is active even on disabled screens.

## Extensibility pointers

- Add new Win32 UI helpers in `src/ui`; keep platform-agnostic helpers (blur math, capture, rendering) in their existing modules to preserve separation.
- For new settings, extend `src/settings.rs` and the dialog controls simultaneously to keep the persisted format, UI state, and runtime behavior in sync.
- The shared `AppState` in `src/state.rs` should still be the single source of truth for display state, password buffer, warnings, and monitor handles.

## Running and testing

- Use `cargo fmt --all` and `cargo check` locally before pushing to make sure formatting and compilation stay clean.
- The Windows workflow expects MSVC/GNU builds; refer to `.github/workflows/windows-build.yml` for the exact targets if you add platform-specific code.
