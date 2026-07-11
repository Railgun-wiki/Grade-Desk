use keyring::Entry;
use reqwest::{header::COOKIE, Client, Url};
use serde::Serialize;
use serde_json::Value;
use tauri::{
    webview::{Cookie, PageLoadEvent},
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

const JWXT_HOST: &str = "jwxt.sysu.edu.cn";
const JWXT_LOGIN: &str = "https://jwxt.sysu.edu.cn/jwxt/api/sso/cas/login?pattern=student-login";
const JWXT_PULL: &str = "https://jwxt.sysu.edu.cn/jwxt/achievement-manage/score-check/getPull";
const SESSION_SERVICE: &str = "edu.sysu.grade-desk";
const SESSION_ACCOUNT: &str = "jwxt-cookie-v1";

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

pub(crate) fn status() -> JwxtStatus {
    match load_cookie_header() {
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
    let handle = app.clone();
    let url: Url = JWXT_LOGIN.parse().map_err(to_message)?;
    WebviewWindowBuilder::new(app, "jwxt-login", WebviewUrl::External(url))
        .title("连接中大教务")
        .inner_size(980.0, 720.0)
        .min_inner_size(720.0, 560.0)
        .on_page_load(move |window, payload| {
            if payload.event() == PageLoadEvent::Finished
                && payload.url().host_str() == Some(JWXT_HOST)
            {
                if persist_window_cookies(&window).is_ok() {
                    let _ = handle.emit(
                        "jwxt-session-updated",
                        JwxtStatus {
                            connected: true,
                            message: "教务会话已保存到 macOS 钥匙串。".into(),
                        },
                    );
                }
            }
        })
        .build()
        .map_err(to_message)?;
    Ok(())
}

pub(crate) async fn verify_session() -> Result<GradeQueryResult, String> {
    let header = load_cookie_header()?;
    let client = Client::new();
    let pull: Value = client
        .get(JWXT_PULL)
        .header(COOKIE, header.clone())
        .send()
        .await
        .map_err(to_message)?
        .json()
        .await
        .map_err(to_message)?;
    ensure_success(&pull)?;
    let train_type = pull
        .pointer("/data/selectTrainType/0/dataNumber")
        .and_then(Value::as_str)
        .ok_or_else(|| "教务系统未返回培养类别。".to_owned())?;
    let url = format!("https://{JWXT_HOST}/jwxt/achievement-manage/score-check/list?trainTypeCode={train_type}&addScoreFlag=true");
    let grades: Value = client
        .get(url)
        .header(COOKIE, header)
        .send()
        .await
        .map_err(to_message)?
        .json()
        .await
        .map_err(to_message)?;
    ensure_success(&grades)?;
    let count = grades
        .get("data")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    Ok(GradeQueryResult {
        course_count: count,
        train_type: train_type.to_owned(),
    })
}

fn persist_window_cookies(window: &tauri::WebviewWindow) -> Result<(), String> {
    let url: Url = format!("https://{JWXT_HOST}/")
        .parse()
        .map_err(to_message)?;
    let cookies = window.cookies_for_url(url).map_err(to_message)?;
    if cookies.is_empty() {
        return Err("尚未获得教务会话 Cookie。".into());
    }
    let serialized: Vec<String> = cookies
        .into_iter()
        .map(|cookie| cookie.to_string())
        .collect();
    session_entry()?
        .set_password(&serde_json::to_string(&serialized).map_err(to_message)?)
        .map_err(to_message)
}

fn load_cookie_header() -> Result<String, String> {
    let encoded = session_entry()?
        .get_password()
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

fn session_entry() -> Result<Entry, String> {
    Entry::new(SESSION_SERVICE, SESSION_ACCOUNT).map_err(to_message)
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
