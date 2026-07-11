use rusqlite::{params, Connection, Result as SqlResult};
use serde::Serialize;
use std::fs;
use tauri::{AppHandle, Manager};

const DATABASE_FILE: &str = "grade-desk.db";
const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Dashboard {
    pub(crate) profile_name: String,
    pub(crate) current_term: String,
    pub(crate) cumulative_gpa: f64,
    pub(crate) earned_credits: f64,
    pub(crate) course_count: i64,
    pub(crate) last_synced_at: String,
}

pub(crate) fn load_dashboard(app: &AppHandle) -> Result<Dashboard, String> {
    let mut connection = open_application_database(app)?;
    migrate(&connection).map_err(to_message)?;
    seed_demo_data(&mut connection).map_err(to_message)?;
    dashboard_from(&connection).map_err(to_message)
}

fn open_application_database(app: &AppHandle) -> Result<Connection, String> {
    let directory = app.path().app_data_dir().map_err(to_message)?;
    fs::create_dir_all(&directory).map_err(to_message)?;
    let connection = Connection::open(directory.join(DATABASE_FILE)).map_err(to_message)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(to_message)?;
    Ok(connection)
}

fn migrate(connection: &Connection) -> SqlResult<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY,
            display_name TEXT NOT NULL,
            school TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS terms (
            id INTEGER PRIMARY KEY,
            profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
            academic_year TEXT NOT NULL,
            semester INTEGER NOT NULL CHECK (semester BETWEEN 1 AND 3),
            train_type TEXT NOT NULL,
            UNIQUE(profile_id, academic_year, semester, train_type)
        );
        CREATE TABLE IF NOT EXISTS courses (
            id INTEGER PRIMARY KEY,
            profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
            course_code TEXT NOT NULL,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            UNIQUE(profile_id, course_code)
        );
        CREATE TABLE IF NOT EXISTS course_attempts (
            id INTEGER PRIMARY KEY,
            course_id INTEGER NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
            term_id INTEGER NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
            class_number TEXT,
            official_grade TEXT,
            numeric_score REAL,
            score_kind TEXT NOT NULL CHECK (score_kind IN ('official_numeric', 'official_grade', 'derived', 'unavailable')),
            grade_point REAL,
            credit REAL NOT NULL CHECK (credit >= 0),
            passed INTEGER NOT NULL CHECK (passed IN (0, 1)),
            recorded_at TEXT NOT NULL,
            UNIQUE(term_id, class_number)
        );
        CREATE TABLE IF NOT EXISTS score_components (
            id INTEGER PRIMARY KEY,
            attempt_id INTEGER NOT NULL REFERENCES course_attempts(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            score REAL,
            weight REAL
        );
        CREATE TABLE IF NOT EXISTS sync_runs (
            id INTEGER PRIMARY KEY,
            profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed')),
            source_version TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS grade_snapshots (
            id INTEGER PRIMARY KEY,
            sync_run_id INTEGER NOT NULL REFERENCES sync_runs(id) ON DELETE CASCADE,
            attempt_id INTEGER NOT NULL REFERENCES course_attempts(id) ON DELETE CASCADE,
            payload_hash TEXT NOT NULL,
            normalized_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        PRAGMA user_version = 1;
        ",
    )?;

    debug_assert_eq!(
        connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i32>(0))?,
        SCHEMA_VERSION
    );
    Ok(())
}

fn seed_demo_data(connection: &mut Connection) -> SqlResult<()> {
    let profile_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))?;
    if profile_count > 0 {
        return Ok(());
    }

    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO profiles (id, display_name, school, created_at) VALUES (1, ?1, ?2, ?3)",
        params!["示例同学", "中山大学", "2026-07-12T00:00:00Z"],
    )?;
    transaction.execute(
        "INSERT INTO terms (id, profile_id, academic_year, semester, train_type) VALUES (1, 1, ?1, 1, ?2)",
        params!["2025-2026", "本科"],
    )?;

    let courses = [
        (1, "CS101", "程序设计基础", "专业必修"),
        (2, "MA101", "高等数学", "公共必修"),
        (3, "EN101", "学术英语", "公共必修"),
        (4, "GE101", "科学与社会", "通识选修"),
    ];
    for (id, code, name, category) in courses {
        transaction.execute(
            "INSERT INTO courses (id, profile_id, course_code, name, category) VALUES (?1, 1, ?2, ?3, ?4)",
            params![id, code, name, category],
        )?;
    }

    let attempts = [
        (1, 1, "CS101-01", "A", None, "official_grade", 4.0, 4.0, 1),
        (
            2,
            2,
            "MA101-01",
            "A-",
            Some(91.0),
            "official_numeric",
            3.7,
            5.0,
            1,
        ),
        (3, 3, "EN101-01", "B+", None, "official_grade", 3.3, 2.0, 1),
        (
            4,
            4,
            "GE101-01",
            "A",
            Some(95.0),
            "official_numeric",
            4.0,
            2.0,
            1,
        ),
    ];
    for (id, course_id, class_number, grade, score, score_kind, grade_point, credit, passed) in
        attempts
    {
        transaction.execute(
            "INSERT INTO course_attempts (id, course_id, term_id, class_number, official_grade, numeric_score, score_kind, grade_point, credit, passed, recorded_at)
             VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, course_id, class_number, grade, score, score_kind, grade_point, credit, passed, "2026-07-12T00:00:00Z"],
        )?;
    }

    transaction.execute(
        "INSERT INTO sync_runs (id, profile_id, started_at, finished_at, status, source_version) VALUES (1, 1, ?1, ?1, 'completed', 'seed-v1')",
        params!["2026-07-12T00:00:00Z"],
    )?;
    transaction.execute(
        "INSERT INTO grade_snapshots (sync_run_id, attempt_id, payload_hash, normalized_json, created_at)
         VALUES (1, 1, ?1, ?2, ?3)",
        params!["seed-cs101-v1", r#"{"officialGrade":"A","scoreKind":"official_grade"}"#, "2026-07-12T00:00:00Z"],
    )?;
    transaction.commit()
}

fn dashboard_from(connection: &Connection) -> SqlResult<Dashboard> {
    connection.query_row(
        "
        SELECT
            p.display_name,
            COALESCE((SELECT academic_year || ' 第' || semester || '学期' FROM terms WHERE profile_id = p.id ORDER BY id DESC LIMIT 1), '暂无学期'),
            COALESCE(SUM(CASE WHEN a.passed = 1 THEN a.credit ELSE 0 END), 0),
            COUNT(a.id),
            COALESCE(SUM(a.grade_point * a.credit) / NULLIF(SUM(a.credit), 0), 0),
            COALESCE((SELECT finished_at FROM sync_runs WHERE profile_id = p.id AND status = 'completed' ORDER BY id DESC LIMIT 1), '')
        FROM profiles p
        LEFT JOIN courses c ON c.profile_id = p.id
        LEFT JOIN course_attempts a ON a.course_id = c.id
        GROUP BY p.id
        ORDER BY p.id
        LIMIT 1
        ",
        [],
        |row| {
            Ok(Dashboard {
                profile_name: row.get(0)?,
                current_term: row.get(1)?,
                earned_credits: row.get(2)?,
                course_count: row.get(3)?,
                cumulative_gpa: row.get(4)?,
                last_synced_at: row.get(5)?,
            })
        },
    )
}

fn to_message(error: impl std::fmt::Display) -> String {
    format!("Unable to open local grade data: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_and_seed_are_idempotent() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&connection).expect("apply schema");
        seed_demo_data(&mut connection).expect("seed data");
        seed_demo_data(&mut connection).expect("repeat seed data");

        let dashboard = dashboard_from(&connection).expect("load dashboard");
        assert_eq!(dashboard.profile_name, "示例同学");
        assert_eq!(dashboard.course_count, 4);
        assert_eq!(dashboard.earned_credits, 13.0);
        assert!((dashboard.cumulative_gpa - 3.78).abs() < 0.01);
    }
}
