use crate::data::{self, RemoteGrade};
use reqwest::{
    header::{ACCEPT, COOKIE, REFERER, USER_AGENT},
    Client, Url,
};
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::{webview::Cookie, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tracing::{debug, info, warn};

const JWXT_HOST: &str = "jwxt.sysu.edu.cn";
const JWXT_LOGIN: &str = "https://jwxt.sysu.edu.cn/jwxt/api/sso/cas/login?pattern=student-login";
const JWXT_PULL: &str = "https://jwxt.sysu.edu.cn/jwxt/achievement-manage/score-check/getPull";
const SESSION_FILE: &str = "jwxt-session.json";
const DIAGNOSTIC_LOG: &str = "jwxt-diagnostics.log";

#[derive(Clone, Copy)]
enum DiagnosticLevel {
    Debug,
    Warn,
}

impl DiagnosticLevel {
    fn label(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
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

pub(crate) fn start_login(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("jwxt-login") {
        window.show().map_err(to_message)?;
        window.set_focus().map_err(to_message)?;
        return Ok(());
    }
    let url: Url = JWXT_LOGIN.parse().map_err(to_message)?;
    WebviewWindowBuilder::new(app, "jwxt-login", WebviewUrl::External(url))
        .title("连接中大教务")
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

pub(crate) async fn verify_session(app: &AppHandle) -> Result<GradeQueryResult, String> {
    let (_, result) = fetch_grades(app).await?;
    Ok(result)
}

pub(crate) async fn sync_grades(app: &AppHandle) -> Result<GradeQueryResult, String> {
    let (records, result) = fetch_grades(app).await?;
    data::import_jwxt_grades(app, records)?;
    Ok(result)
}

async fn fetch_grades(app: &AppHandle) -> Result<(Vec<RemoteGrade>, GradeQueryResult), String> {
    let header = load_cookie_header(app)?;
    let client = Client::new();
    let pull = get_json(app, &client, JWXT_PULL, &header, "getPull").await?;
    ensure_success(&pull)?;
    let train_type = pull
        .pointer("/data/selectTrainType/0/dataNumber")
        .and_then(Value::as_str)
        .ok_or_else(|| "教务系统未返回培养类别。".to_owned())?;
    let url = format!("https://{JWXT_HOST}/jwxt/achievement-manage/score-check/list?trainTypeCode={train_type}&addScoreFlag=true");
    let grades = get_json(app, &client, &url, &header, "score-check/list").await?;
    ensure_success(&grades)?;
    let records = grades
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "教务系统未返回成绩列表。".to_owned())?
        .iter()
        .filter_map(parse_grade)
        .collect::<Vec<_>>();
    let result = GradeQueryResult {
        course_count: records.len(),
        train_type: train_type.to_owned(),
    };
    Ok((records, result))
}

async fn get_json(
    app: &AppHandle,
    client: &Client,
    url: &str,
    cookie: &str,
    operation: &str,
) -> Result<Value, String> {
    let response = client
        .get(url)
        .header(COOKIE, cookie)
        .header(REFERER, format!("https://{JWXT_HOST}/"))
        .header(ACCEPT, "application/json, text/plain, */*")
        .header(USER_AGENT, "Mozilla/5.0 (Macintosh; ARM Mac OS X 15_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15")
        .send()
        .await
        .map_err(to_message)?;
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let body = response.text().await.map_err(to_message)?;
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
