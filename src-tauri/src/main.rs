// ============================================================
// main.rs  — Binary entry point (calls lib.rs)
// ============================================================

// Prevent console window from appearing on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    connex_lib::run()
}
