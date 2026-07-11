mod data;

use data::{CourseAttempt, CourseDetail, Dashboard};
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

#[tauri::command]
fn list_course_attempts(app: tauri::AppHandle) -> Result<Vec<CourseAttempt>, String> {
    data::list_course_attempts(&app)
}

#[tauri::command]
fn get_course_detail(app: tauri::AppHandle, attempt_id: i64) -> Result<CourseDetail, String> {
    data::get_course_detail(&app, attempt_id)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            application_status,
            get_dashboard,
            list_course_attempts,
            get_course_detail
        ])
        .run(tauri::generate_context!())
        .expect("error while running Grade Desk");
}
