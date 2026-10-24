mod data;
mod jwxt;
mod logging;
mod platform;

use data::{
    AnalysisOverview, ArchiveResult, ChangeRecord, CourseAttempt, CourseDetail, Dashboard,
    ExportFormat, ExportReceipt, SyncRun, TermOption,
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    name: &'static str,
    version: &'static str,
    storage_mode: &'static str,
    os: &'static str,
}

#[tauri::command]
fn application_status() -> AppStatus {
    AppStatus {
        name: "Grade Desk",
        version: env!("CARGO_PKG_VERSION"),
        storage_mode: "local-only",
        os: platform::Platform::current().as_str(),
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
fn list_terms(app: tauri::AppHandle) -> Result<Vec<TermOption>, String> {
    data::list_terms(&app)
}

#[tauri::command]
fn get_analysis_overview(app: tauri::AppHandle) -> Result<AnalysisOverview, String> {
    data::load_analysis_overview(&app)
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
async fn start_jwxt_login(app: tauri::AppHandle) -> Result<(), String> {
    jwxt::start_login(&app).await
}

#[tauri::command]
async fn save_jwxt_session(app: tauri::AppHandle) -> Result<jwxt::JwxtStatus, String> {
    jwxt::save_login_window_session(&app).await
}

#[tauri::command]
async fn verify_jwxt_session(app: tauri::AppHandle) -> Result<jwxt::SessionVerification, String> {
    jwxt::verify_session(&app).await
}

#[tauri::command]
async fn sync_jwxt_grades(
    app: tauri::AppHandle,
    method: jwxt::GradeQueryMethod,
) -> Result<jwxt::GradeQueryResult, String> {
    jwxt::sync_grades(&app, method).await
}

#[tauri::command]
async fn probe_jwxt_numeric_score(
    app: tauri::AppHandle,
    attempt_id: i64,
) -> Result<jwxt::NumericProbeResult, String> {
    jwxt::probe_numeric_score(&app, attempt_id).await
}

#[tauri::command]
async fn query_jwxt_rank_summary(app: tauri::AppHandle) -> Result<jwxt::RankSummary, String> {
    jwxt::query_rank_summary(&app).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    tauri::Builder::default()
        .setup(|app| {
            platform::current().configure_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            application_status,
            get_dashboard,
            list_course_attempts,
            list_terms,
            get_analysis_overview,
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
            sync_jwxt_grades,
            probe_jwxt_numeric_score,
            query_jwxt_rank_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running Grade Desk");
}

#[cfg(test)]
mod tests {
    #[test]
    fn application_status_reports_the_current_platform() {
        let status = super::application_status();
        assert_eq!(status.os, crate::platform::Platform::current().as_str());
        assert_eq!(status.storage_mode, "local-only");
    }
}
