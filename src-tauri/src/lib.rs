mod data;

use data::Dashboard;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    name: &'static str,
    version: &'static str,
    storage_mode: &'static str,
}

#[tauri::command]
fn application_status() -> AppStatus {
    AppStatus {
        name: "Grade Desk",
        version: env!("CARGO_PKG_VERSION"),
        storage_mode: "local-only",
    }
}

#[tauri::command]
fn get_dashboard(app: tauri::AppHandle) -> Result<Dashboard, String> {
    data::load_dashboard(&app)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![application_status, get_dashboard])
        .run(tauri::generate_context!())
        .expect("error while running Grade Desk");
}
