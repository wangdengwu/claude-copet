// The pet shell. Window chrome/transparency/always-on-top live in
// tauri.conf.json so later slices can extend behavior without touching this.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
