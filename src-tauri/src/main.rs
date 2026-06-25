// Prevents an extra console window on Windows in release; no effect elsewhere.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    claude_copet_lib::run()
}
