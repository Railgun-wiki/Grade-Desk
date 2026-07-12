use crate::data::{self, RemoteGrade};
use reqwest::{
    header::{ACCEPT, COOKIE, REFERER, USER_AGENT},
    Client, RequestBuilder, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::{webview::Cookie, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tracing::{debug, info, warn};

const JWXT_HOST: &str = "jwxt.sysu.edu.cn";
const JWXT_LOGIN: &str = "https://jwxt.sysu.edu.cn/jwxt/api/sso/cas/login?pattern=student-login";
const JWXT_PULL: &str = "https://jwxt.sysu.edu.cn/jwxt/achievement-manage/score-check/getPull";
const JWXT_SCORE_LIST: &str = "https://jwxt.sysu.edu.cn/jwxt/achievement-manage/score-check/list";
const JWXT_RANK_SUMMARY: &str =
    "https://jwxt.sysu.edu.cn/jwxt/achievement-manage/score-check/getSortByYear";
const JWXT_ACHIEVEMENT_SEARCH: &str =
    "https://jwxt.sysu.edu.cn/jwxt/achievement-manage/achievement/selfPageList";
const JWXT_NUMERIC_PROBE: &str = "https://jwxt.sysu.edu.cn/jwxt/gradua-degree/graduatemsg/studentsGraduationExamination/studentCourse";
const SESSION_FILE: &str = "jwxt-session.json";
const DIAGNOSTIC_LOG: &str = "jwxt-diagnostics.log";

#[derive(Clone, Copy)]
enum DiagnosticLevel {
    Debug,
    Info,
    Warn,
}

impl DiagnosticLevel {
    fn label(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JwxtStatus {
    pub(crate) connected: bool,
    pub(crate) message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GradeQueryResult {
    pub(crate) course_count: usize,
    pub(crate) train_type: String,
    pub(crate) method: GradeQueryMethod,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub(crate) enum GradeQueryMethod {
    OfficialList,
    AchievementSearch,
}

impl GradeQueryMethod {
    fn label(self) -> &'static str {
        match self {
            Self::OfficialList => "官方成绩单",
            Self::AchievementSearch => "课程成绩检索",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionVerification {
    pub(crate) train_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NumericProbeResult {
    pub(crate) numeric_score: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RankSummary {
    pub(crate) train_type: String,
    pub(crate) total_rank: Option<String>,
    pub(crate) term_rank: Option<String>,
    pub(crate) total_students: Option<String>,
    pub(crate) cumulative_gpa: Option<String>,
    pub(crate) term_gpa: Option<String>,
    pub(crate) earned_credits: Option<String>,
}

pub(crate) fn status(app: &AppHandle) -> JwxtStatus {
    match load_cookie_header(app) {
        Ok(header) if !header.is_empty() => JwxtStatus {
            connected: true,
            message: "已保存教务会话；可验证或同步。".into(),
        },
        _ => JwxtStatus {
            connected: false,
            message: "尚未连接教务系统。".into(),
        },
    }
}

/// Opens the login window from Tauri's asynchronous command path.
///
/// WebView2 can deadlock when a `WebviewWindow` is created by a synchronous
/// command on Windows, so callers must await this function.
pub(crate) async fn start_login(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("jwxt-login") {
        window.show().map_err(to_message)?;
        window.set_focus().map_err(to_message)?;
        return Ok(());
    }
    let url: Url = JWXT_LOGIN.parse().map_err(to_message)?;
    WebviewWindowBuilder::new(app, "jwxt-login", WebviewUrl::External(url))
        .title("连接教务系统")
        .inner_size(980.0, 720.0)
        .min_inner_size(720.0, 560.0)
        .build()
        .map_err(to_message)?;
    Ok(())
}

pub(crate) fn save_login_window_session(app: &AppHandle) -> Result<JwxtStatus, String> {
    let window = app
        .get_webview_window("jwxt-login")
        .ok_or_else(|| "未找到教务登录窗口。请先打开登录并完成认证。".to_owned())?;
    persist_window_cookies(app, &window)?;
    info!("JWXT session persisted locally after explicit user action");
    Ok(JwxtStatus {
        connected: true,
        message: "教务会话已保存到本机。".into(),
    })
}

pub(crate) async fn verify_session(app: &AppHandle) -> Result<SessionVerification, String> {
    let train_type = fetch_train_type(app).await?;
    info!(train_type, "JWXT session verification succeeded");
    Ok(SessionVerification { train_type })
}

pub(crate) async fn sync_grades(
    app: &AppHandle,
    method: GradeQueryMethod,
) -> Result<GradeQueryResult, String> {
    info!(
        method = method.label(),
        "JWXT grade query requested by user"
    );
    let (records, result) = fetch_grades(app, method).await?;
    data::import_jwxt_grades(app, records)?;
    Ok(result)
}

pub(crate) async fn probe_numeric_score(
    app: &AppHandle,
    attempt_id: i64,
) -> Result<NumericProbeResult, String> {
    info!("JWXT numeric-score probe started");
    write_diagnostic(app, DiagnosticLevel::Info, "numeric-score-probe: started");
    let target = data::numeric_probe_target(app, attempt_id)
        .map_err(|error| probe_failure(app, "numeric-score-probe/target", error))?;
    let candidates = numeric_score_candidates(&target.official_grade)
        .map_err(|error| probe_failure(app, "numeric-score-probe/candidates", error))?;
    let header = load_cookie_header(app)
        .map_err(|error| probe_failure(app, "numeric-score-probe/session", error))?;
    let client = Client::new();
    info!(
        attempts = candidates.len(),
        "JWXT numeric-score probe requested by user"
    );

    for score in candidates {
        let response = get_json(
            app,
            jwxt_post(&client, JWXT_NUMERIC_PROBE, &header).json(&serde_json::json!({
                "pageNo": 1,
                "pageSize": 10,
                "total": true,
                "param": {
                    "achievementCourseNumber": target.course_number.as_str(),
                    "beforeAchievementPoint": score,
                    "afterAchievementPoint": score,
                    "cultureTypeCode": "01"
                }
            })),
            "numeric-score-probe",
        )
        .await
        .map_err(|error| probe_failure(app, "numeric-score-probe/request", error))?;
        ensure_success(&response)
            .map_err(|error| probe_failure(app, "numeric-score-probe/response", error))?;
        if response
            .pointer("/data/total")
            .and_then(value_to_number)
            .is_some_and(|total| total > 0.0)
        {
            data::save_verified_numeric_score(app, attempt_id, score)
                .map_err(|error| probe_failure(app, "numeric-score-probe/save", error))?;
            info!("JWXT numeric-score probe confirmed and saved locally");
            return Ok(NumericProbeResult {
                numeric_score: score,
            });
        }
    }

    Err(probe_failure(
        app,
        "numeric-score-probe/result",
        "教务未确认该课程的数值成绩；未修改本地记录。".into(),
    ))
}

pub(crate) async fn query_rank_summary(app: &AppHandle) -> Result<RankSummary, String> {
    let header = load_cookie_header(app)?;
    let client = Client::new();
    let train_type = fetch_train_type(app).await?;
    info!("JWXT rank summary requested by user");
    let url = format!("{JWXT_RANK_SUMMARY}?trainTypeCode={train_type}&addScoreFlag=true");
    let response = get_json(
        app,
        jwxt_get(&client, &url, &header),
        "score-check/getSortByYear",
    )
    .await?;
    ensure_success(&response)?;
    parse_rank_summary(&response, train_type)
}

async fn fetch_train_type(app: &AppHandle) -> Result<String, String> {
    let header = load_cookie_header(app)?;
    let client = Client::new();
    let pull = get_json(app, jwxt_get(&client, JWXT_PULL, &header), "getPull").await?;
    ensure_success(&pull)?;
    pull.pointer("/data/selectTrainType/0/dataNumber")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "教务系统未返回培养类别。".to_owned())
}

async fn fetch_grades(
    app: &AppHandle,
    method: GradeQueryMethod,
) -> Result<(Vec<RemoteGrade>, GradeQueryResult), String> {
    let header = load_cookie_header(app)?;
    let client = Client::new();
    let train_type = fetch_train_type(app).await?;
    let grades = match method {
        GradeQueryMethod::OfficialList => {
            let url = format!("{JWXT_SCORE_LIST}?trainTypeCode={train_type}&addScoreFlag=true");
            get_json(app, jwxt_get(&client, &url, &header), "score-check/list").await?
        }
        GradeQueryMethod::AchievementSearch => {
            get_json(
                app,
                jwxt_post(&client, JWXT_ACHIEVEMENT_SEARCH, &header).json(&serde_json::json!({
                    "pageNo": 1,
                    "pageSize": 500,
                    "total": true,
                    "param": {}
                })),
                "achievement/selfPageList",
            )
            .await?
        }
    };
    ensure_success(&grades)?;
    let items = match method {
        GradeQueryMethod::OfficialList => grades.get("data").and_then(Value::as_array),
        GradeQueryMethod::AchievementSearch => {
            grades.pointer("/data/rows").and_then(Value::as_array)
        }
    }
    .ok_or_else(|| "教务系统未返回可导入的成绩列表。".to_owned())?;
    let records = items
        .iter()
        .filter_map(|grade| match method {
            GradeQueryMethod::OfficialList => parse_grade(grade),
            GradeQueryMethod::AchievementSearch => parse_achievement_grade(grade),
        })
        .collect::<Vec<_>>();
    let result = GradeQueryResult {
        course_count: records.len(),
        train_type,
        method,
    };
    Ok((records, result))
}

async fn get_json(
    app: &AppHandle,
    request: RequestBuilder,
    operation: &str,
) -> Result<Value, String> {
    let response = request.send().await.map_err(|error| {
        let message = to_message(error);
        warn!(
            operation,
            reason = message.as_str(),
            "JWXT request failed before a response"
        );
        write_diagnostic(
            app,
            DiagnosticLevel::Warn,
            &format!("{operation}: request failed: {message}"),
        );
        message
    })?;
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let body = response.text().await.map_err(|error| {
        let message = to_message(error);
        warn!(
            operation,
            reason = message.as_str(),
            "JWXT response body read failed"
        );
        write_diagnostic(
            app,
            DiagnosticLevel::Warn,
            &format!("{operation}: body read failed: {message}"),
        );
        message
    })?;
    let body_kind = if body.contains("Access Forbidden") {
        "access-forbidden"
    } else if body.trim_start().starts_with('<') {
        "html"
    } else if body.trim_start().starts_with('{') || body.trim_start().starts_with('[') {
        "json-like"
    } else {
        "other"
    };
    debug!(
        operation,
        %status,
        content_type,
        body_kind,
        body_bytes = body.len(),
        "JWXT response received"
    );
    write_diagnostic(
        app,
        DiagnosticLevel::Debug,
        &format!(
            "{operation}: status={status} content-type={content_type} body={body_kind} bytes={}",
            body.len()
        ),
    );
    let payload: Value = serde_json::from_str(&body).map_err(|error| {
        format!(
            "JWXT {operation} 未返回 JSON（{content_type}，{body_kind}，{} bytes）：{error}。详情已写入本地诊断日志。",
            body.len()
        )
    })?;
    if !status.is_success() {
        let business_code = payload
            .get("code")
            .map(Value::to_string)
            .unwrap_or_else(|| "missing".to_owned());
        warn!(
            operation,
            %status,
            json_code = business_code.as_str(),
            "accepting JSON response with non-success HTTP status"
        );
        write_diagnostic(
            app,
            DiagnosticLevel::Warn,
            &format!(
                "{operation}: accepting non-success HTTP status={status}; json-code={business_code}"
            ),
        );
    }
    Ok(payload)
}

fn jwxt_request(request: RequestBuilder, cookie: &str) -> RequestBuilder {
    request
        .header(COOKIE, cookie)
        .header(REFERER, format!("https://{JWXT_HOST}/"))
        .header(ACCEPT, "application/json, text/plain, */*")
        .header(USER_AGENT, "Mozilla/5.0 (Macintosh; ARM Mac OS X 15_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15")
}

fn jwxt_get(client: &Client, url: &str, cookie: &str) -> RequestBuilder {
    jwxt_request(client.get(url), cookie)
}

fn jwxt_post(client: &Client, url: &str, cookie: &str) -> RequestBuilder {
    jwxt_request(client.post(url), cookie)
}

fn parse_grade(value: &Value) -> Option<RemoteGrade> {
    let course_name = value.get("scoCourseName")?.as_str()?.trim().to_owned();
    let class_number = value
        .get("teachClassNumber")
        .or_else(|| value.get("scoCourseNumber"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-class")
        .to_owned();
    let course_code = value
        .get("scoCourseNumber")
        .and_then(Value::as_str)
        .unwrap_or(&class_number)
        .to_owned();
    Some(RemoteGrade {
        course_name,
        course_code,
        category: value
            .get("scoCourseCategoryName")
            .and_then(Value::as_str)
            .unwrap_or("未分类")
            .to_owned(),
        class_number,
        official_grade: value
            .get("scoFinalScore")
            .and_then(Value::as_str)
            .map(str::to_owned),
        grade_point: value
            .get("scoPoint")
            .and_then(Value::as_str)
            .and_then(|point| point.parse().ok()),
        credit: value
            .get("scoCredit")
            .and_then(Value::as_str)
            .and_then(|credit| credit.parse().ok())
            .unwrap_or(0.0),
        academic_year: value
            .get("scoSchoolYear")
            .and_then(Value::as_str)
            .unwrap_or("未知学年")
            .to_owned(),
        semester: value
            .get("scoSemester")
            .and_then(Value::as_i64)
            .unwrap_or(1),
        passed: value
            .get("accessFlag")
            .and_then(Value::as_str)
            .map(|status| status.contains('过'))
            .unwrap_or(true),
    })
}

fn parse_achievement_grade(value: &Value) -> Option<RemoteGrade> {
    let course_name = value.get("courseName")?.as_str()?.trim().to_owned();
    let class_number = value
        .get("classesNum")
        .or_else(|| value.get("courseNum"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-class")
        .to_owned();
    let course_code = value
        .get("courseNum")
        .and_then(Value::as_str)
        .unwrap_or(&class_number)
        .to_owned();
    let term = value
        .get("schoolSemester")
        .and_then(Value::as_str)
        .unwrap_or("未知学年");
    let (academic_year, semester) = split_school_semester(term);
    Some(RemoteGrade {
        course_name,
        course_code,
        category: value
            .get("courseCategoryName")
            .and_then(Value::as_str)
            .unwrap_or("未分类")
            .to_owned(),
        class_number,
        official_grade: value
            .get("finalAchievementStr")
            .or_else(|| value.get("totalAchievement"))
            .and_then(value_to_text),
        grade_point: value.get("achievementPoint").and_then(value_to_number),
        credit: value.get("credit").and_then(value_to_number).unwrap_or(0.0),
        academic_year,
        semester,
        passed: true,
    })
}

fn split_school_semester(value: &str) -> (String, i64) {
    let Some((year, semester)) = value.rsplit_once('-') else {
        return (value.to_owned(), 1);
    };
    (year.to_owned(), semester.parse().unwrap_or(1))
}

fn numeric_score_candidates(official_grade: &str) -> Result<Vec<i64>, String> {
    let grade = official_grade.trim();
    let base = match grade.chars().next() {
        Some('A') => 100,
        Some('B') => 90,
        Some('C') => 80,
        Some('D') => 70,
        Some('F') => 60,
        _ => return Err("该官方成绩不支持数值探测。".into()),
    };
    let ceiling = base - if grade.chars().count() == 2 { 0 } else { 6 };
    if ceiling < 60 {
        return Err("该官方成绩没有可探测的数值区间。".into());
    }
    Ok((60..=ceiling).rev().take(41).collect())
}

fn value_to_text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_i64().map(|number| number.to_string()))
        .or_else(|| value.as_f64().map(|number| number.to_string()))
}

fn value_to_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|number| number.parse().ok()))
}

fn parse_rank_summary(response: &Value, train_type: String) -> Result<RankSummary, String> {
    let data = response
        .get("data")
        .ok_or_else(|| "教务系统未返回排名统计。".to_owned())?;
    let total = data
        .get("compulsorySelectTotal")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    let term = data
        .get("compulsorySelectList")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    Ok(RankSummary {
        train_type,
        total_rank: total
            .and_then(|item| item.get("rank"))
            .and_then(value_to_text),
        term_rank: term
            .and_then(|item| item.get("rank"))
            .and_then(value_to_text),
        total_students: data.get("stuTotal").and_then(value_to_text),
        cumulative_gpa: total
            .and_then(|item| item.get("vegPoint"))
            .and_then(value_to_text),
        term_gpa: term
            .and_then(|item| item.get("vegPoint"))
            .and_then(value_to_text),
        earned_credits: total
            .and_then(|item| item.get("totalCredit"))
            .and_then(value_to_text),
    })
}

fn persist_window_cookies(app: &AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    let url: Url = format!("https://{JWXT_HOST}/")
        .parse()
        .map_err(to_message)?;
    let mut cookies = window.cookies_for_url(url).map_err(to_message)?;
    if cookies.is_empty() {
        cookies = window
            .cookies()
            .map_err(to_message)?
            .into_iter()
            .filter(|cookie| {
                cookie
                    .domain()
                    .is_some_and(|domain| domain.trim_start_matches('.').ends_with(JWXT_HOST))
            })
            .collect();
    }
    if cookies.is_empty() {
        return Err("尚未获得教务会话 Cookie。".into());
    }
    let serialized: Vec<String> = cookies
        .into_iter()
        .map(|cookie| cookie.to_string())
        .collect();
    let path = session_file(app)?;
    fs::write(
        &path,
        serde_json::to_string(&serialized).map_err(to_message)?,
    )
    .map_err(to_message)?;
    restrict_file_permissions(&path)?;
    Ok(())
}

fn load_cookie_header(app: &AppHandle) -> Result<String, String> {
    let encoded = fs::read_to_string(session_file(app)?)
        .map_err(|_| "尚未保存教务会话，请先在应用内完成登录。".to_owned())?;
    let cookies: Vec<String> = serde_json::from_str(&encoded).map_err(to_message)?;
    let pairs = cookies
        .into_iter()
        .filter_map(|raw| {
            Cookie::parse(raw)
                .ok()
                .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
        })
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        Err("已保存的教务会话无效，请重新登录。".into())
    } else {
        Ok(pairs.join("; "))
    }
}

fn session_file(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(to_message)?;
    fs::create_dir_all(&directory).map_err(to_message)?;
    Ok(directory.join(SESSION_FILE))
}

fn write_diagnostic(app: &AppHandle, level: DiagnosticLevel, message: &str) {
    let Ok(directory) = app.path().app_data_dir() else {
        return;
    };
    if fs::create_dir_all(&directory).is_err() {
        return;
    }
    let line = format!("{} {} {message}\n", chrono_free_timestamp(), level.label());
    let path = directory.join(DIAGNOSTIC_LOG);
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
    let _ = restrict_file_permissions(&path);
}

fn probe_failure(app: &AppHandle, stage: &str, error: String) -> String {
    warn!(
        stage,
        reason = error.as_str(),
        "JWXT numeric-score probe failed"
    );
    write_diagnostic(
        app,
        DiagnosticLevel::Warn,
        &format!("{stage}: failed: {error}"),
    );
    error
}

fn chrono_free_timestamp() -> String {
    format!(
        "unix:{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

#[cfg(unix)]
fn restrict_file_permissions(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(to_message)
}

#[cfg(not(unix))]
fn restrict_file_permissions(_: &PathBuf) -> Result<(), String> {
    Ok(())
}

fn ensure_success(response: &Value) -> Result<(), String> {
    match response.get("code").and_then(Value::as_i64) {
        Some(200) => Ok(()),
        Some(53000007) => Err("教务会话已失效，请重新在应用内登录。".into()),
        _ => Err(response
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("教务查询失败。")
            .to_owned()),
    }
}

fn to_message(error: impl std::fmt::Display) -> String {
    format!("教务连接失败：{error}")
}

#[cfg(test)]
mod tests {
    use super::split_school_semester;

    #[test]
    fn splits_jwxt_school_semester() {
        assert_eq!(
            split_school_semester("2025-2026-1"),
            ("2025-2026".into(), 1)
        );
        assert_eq!(split_school_semester("2025-1"), ("2025".into(), 1));
        assert_eq!(split_school_semester("unknown"), ("unknown".into(), 1));
    }

    #[test]
    fn bounds_numeric_score_candidates() {
        assert_eq!(super::numeric_score_candidates("A+").unwrap().len(), 41);
        assert_eq!(
            super::numeric_score_candidates("B").unwrap(),
            (60..=84).rev().collect::<Vec<_>>()
        );
        assert!(super::numeric_score_candidates("合格").is_err());
    }

    #[test]
    fn parses_rank_summary() {
        let response = serde_json::json!({
            "data": {
                "compulsorySelectTotal": [{"rank": "12", "vegPoint": "3.82", "totalCredit": "90"}],
                "compulsorySelectList": [{"rank": "4", "vegPoint": "3.95"}],
                "stuTotal": "100"
            }
        });
        let summary = super::parse_rank_summary(&response, "01".into()).unwrap();
        assert_eq!(summary.total_rank.as_deref(), Some("12"));
        assert_eq!(summary.term_rank.as_deref(), Some("4"));
        assert_eq!(summary.cumulative_gpa.as_deref(), Some("3.82"));
    }
}
