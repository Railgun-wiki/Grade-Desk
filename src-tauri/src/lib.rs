mod data;
mod jwxt;
mod logging;

use data::{
    ArchiveResult, ChangeRecord, CourseAttempt, CourseDetail, Dashboard, ExportFormat,
    ExportReceipt, SyncRun,
};
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

#[tauri::command]
fn archive_current_data(app: tauri::AppHandle) -> Result<ArchiveResult, String> {
    data::archive_current_data(&app)
}

#[tauri::command]
fn list_sync_runs(app: tauri::AppHandle) -> Result<Vec<SyncRun>, String> {
    data::list_sync_runs(&app)
}

#[tauri::command]
fn list_pending_changes(app: tauri::AppHandle) -> Result<Vec<ChangeRecord>, String> {
    data::list_pending_changes(&app)
}

#[tauri::command]
fn review_pending_changes(app: tauri::AppHandle) -> Result<usize, String> {
    data::review_pending_changes(&app)
}

#[tauri::command]
fn export_grade_data(app: tauri::AppHandle, format: ExportFormat) -> Result<ExportReceipt, String> {
    data::export_grade_data(&app, format)
}

#[tauri::command]
fn clear_local_data(app: tauri::AppHandle) -> Result<(), String> {
    data::clear_local_data(&app)
}

#[tauri::command]
fn jwxt_status(app: tauri::AppHandle) -> jwxt::JwxtStatus {
    jwxt::status(&app)
}

#[tauri::command]
fn start_jwxt_login(app: tauri::AppHandle) -> Result<(), String> {
    jwxt::start_login(&app)
}

#[tauri::command]
fn save_jwxt_session(app: tauri::AppHandle) -> Result<jwxt::JwxtStatus, String> {
    jwxt::save_login_window_session(&app)
}

#[tauri::command]
async fn verify_jwxt_session(app: tauri::AppHandle) -> Result<jwxt::GradeQueryResult, String> {
    jwxt::verify_session(&app).await
}

#[tauri::command]
async fn sync_jwxt_grades(app: tauri::AppHandle) -> Result<jwxt::GradeQueryResult, String> {
    jwxt::sync_grades(&app).await
}

pub fn run() {
    logging::init();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            application_status,
            get_dashboard,
            list_course_attempts,
            get_course_detail,
            archive_current_data,
            list_sync_runs,
            list_pending_changes,
            review_pending_changes,
            export_grade_data,
            clear_local_data,
            jwxt_status,
            start_jwxt_login,
            save_jwxt_session,
            verify_jwxt_session,
            sync_jwxt_grades
        ])
        .run(tauri::generate_context!())
        .expect("error while running Grade Desk");
}
