use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const DATABASE_FILE: &str = "grade-desk.db";
const SCHEMA_VERSION: i32 = 3;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Dashboard {
    pub(crate) profile_name: String,
    pub(crate) current_term: String,
    pub(crate) all_gpa: f64,
    pub(crate) professional_gpa: f64,
    pub(crate) earned_credits: f64,
    pub(crate) course_count: i64,
    pub(crate) last_synced_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CourseAttempt {
    pub(crate) id: i64,
    pub(crate) course_name: String,
    pub(crate) course_code: String,
    pub(crate) category: String,
    pub(crate) official_grade: Option<String>,
    pub(crate) numeric_score: Option<f64>,
    pub(crate) score_kind: String,
    pub(crate) grade_point: Option<f64>,
    pub(crate) credit: f64,
    pub(crate) passed: bool,
    pub(crate) term: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TermOption {
    pub(crate) id: i64,
    pub(crate) label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TermTrend {
    pub(crate) term: String,
    pub(crate) gpa: Option<f64>,
    pub(crate) earned_credits: f64,
    pub(crate) course_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CourseContribution {
    pub(crate) attempt_id: i64,
    pub(crate) course_name: String,
    pub(crate) course_code: String,
    pub(crate) credit: f64,
    pub(crate) grade_point: f64,
    pub(crate) contribution: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScoreDistributionBin {
    pub(crate) label: String,
    pub(crate) count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnalysisDataQuality {
    pub(crate) numeric_count: i64,
    pub(crate) grade_only_count: i64,
    pub(crate) pass_fail_count: i64,
    pub(crate) unavailable_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnalysisOverview {
    pub(crate) trends: Vec<TermTrend>,
    pub(crate) contributions: Vec<CourseContribution>,
    pub(crate) distribution: Vec<ScoreDistributionBin>,
    pub(crate) data_quality: AnalysisDataQuality,
    pub(crate) as_of: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScoreComponent {
    pub(crate) name: String,
    pub(crate) score: Option<f64>,
    pub(crate) weight: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CourseDetail {
    #[serde(flatten)]
    pub(crate) attempt: CourseAttempt,
    pub(crate) term: String,
    pub(crate) class_number: Option<String>,
    pub(crate) components: Vec<ScoreComponent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncRun {
    pub(crate) id: i64,
    pub(crate) finished_at: String,
    pub(crate) source_version: String,
    pub(crate) snapshot_count: i64,
    pub(crate) change_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeRecord {
    pub(crate) id: i64,
    pub(crate) course_name: String,
    pub(crate) course_code: String,
    pub(crate) detected_at: String,
    pub(crate) change_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveResult {
    pub(crate) sync_run_id: i64,
    pub(crate) snapshot_count: usize,
    pub(crate) changes_detected: usize,
    pub(crate) finished_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportReceipt {
    pub(crate) format: String,
    pub(crate) path: String,
    pub(crate) record_count: usize,
}

#[derive(Debug)]
pub(crate) struct RemoteGrade {
    pub(crate) course_name: String,
    pub(crate) course_code: String,
    pub(crate) category: String,
    pub(crate) class_number: String,
    pub(crate) official_grade: Option<String>,
    pub(crate) grade_point: Option<f64>,
    pub(crate) credit: f64,
    pub(crate) academic_year: String,
    pub(crate) semester: i64,
    pub(crate) passed: bool,
}

#[derive(Debug)]
pub(crate) struct NumericProbeTarget {
    pub(crate) course_number: String,
    pub(crate) official_grade: String,
}

pub(crate) fn load_dashboard(app: &AppHandle) -> Result<Dashboard, String> {
    let connection = initialized_database(app)?;
    dashboard_from(&connection).map_err(to_message)
}

pub(crate) fn list_course_attempts(app: &AppHandle) -> Result<Vec<CourseAttempt>, String> {
    let connection = initialized_database(app)?;
    attempts_from(&connection).map_err(to_message)
}

pub(crate) fn list_terms(app: &AppHandle) -> Result<Vec<TermOption>, String> {
    let connection = initialized_database(app)?;
    let mut statement = connection.prepare(
        "SELECT id, academic_year || ' 第' || semester || '学期' FROM terms WHERE profile_id = 1 ORDER BY academic_year DESC, semester DESC",
    ).map_err(to_message)?;
    let terms = statement.query_map([], |row| Ok(TermOption { id: row.get(0)?, label: row.get(1)? }))
        .map_err(to_message)?.collect::<SqlResult<Vec<_>>>().map_err(to_message)?;
    Ok(terms)
}

pub(crate) fn load_analysis_overview(app: &AppHandle) -> Result<AnalysisOverview, String> {
    let connection = initialized_database(app)?;
    analysis_overview_from(&connection).map_err(to_message)
}

pub(crate) fn get_course_detail(app: &AppHandle, attempt_id: i64) -> Result<CourseDetail, String> {
    let connection = initialized_database(app)?;
    course_detail_from(&connection, attempt_id).map_err(to_message)
}

pub(crate) fn numeric_probe_target(
    app: &AppHandle,
    attempt_id: i64,
) -> Result<NumericProbeTarget, String> {
    let connection = initialized_database(app)?;
    let row = connection
        .query_row(
            "SELECT c.course_code, a.official_grade, a.numeric_score FROM course_attempts a JOIN courses c ON c.id = a.course_id WHERE a.id = ?1",
            params![attempt_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(to_message)?
        .ok_or_else(|| "未找到该课程记录。".to_owned())?;
    let (course_number, official_grade, numeric_score) = row;
    if numeric_score.is_some() {
        return Err("该课程已有已验证的教务数值。".into());
    }
    Ok(NumericProbeTarget {
        course_number: course_number.ok_or_else(|| "该课程缺少课程号，无法探测。".to_owned())?,
        official_grade: official_grade
            .ok_or_else(|| "该课程不是等级制成绩，无法探测。".to_owned())?,
    })
}

pub(crate) fn save_verified_numeric_score(
    app: &AppHandle,
    attempt_id: i64,
    score: i64,
) -> Result<(), String> {
    let mut connection = initialized_database(app)?;
    let updated = connection
        .execute(
            "UPDATE course_attempts SET numeric_score = ?1, score_kind = 'official_numeric', recorded_at = ?2 WHERE id = ?3 AND numeric_score IS NULL",
            params![score, now_timestamp(), attempt_id],
        )
        .map_err(to_message)?;
    if updated == 0 {
        return Err("该课程已变更或已有教务数值，请刷新后重试。".into());
    }
    archive_from(&mut connection).map_err(to_message)?;
    Ok(())
}

pub(crate) fn archive_current_data(app: &AppHandle) -> Result<ArchiveResult, String> {
    let mut connection = initialized_database(app)?;
    archive_from(&mut connection).map_err(to_message)
}

pub(crate) fn list_sync_runs(app: &AppHandle) -> Result<Vec<SyncRun>, String> {
    let connection = initialized_database(app)?;
    sync_runs_from(&connection).map_err(to_message)
}

pub(crate) fn list_pending_changes(app: &AppHandle) -> Result<Vec<ChangeRecord>, String> {
    let connection = initialized_database(app)?;
    pending_changes_from(&connection).map_err(to_message)
}

pub(crate) fn review_pending_changes(app: &AppHandle) -> Result<usize, String> {
    let connection = initialized_database(app)?;
    connection
        .execute(
            "UPDATE grade_changes SET reviewed_at = ?1 WHERE reviewed_at IS NULL",
            params![now_timestamp()],
        )
        .map_err(to_message)
}

pub(crate) fn export_grade_data(
    app: &AppHandle,
    format: ExportFormat,
) -> Result<ExportReceipt, String> {
    let connection = initialized_database(app)?;
    let attempts = attempts_from(&connection).map_err(to_message)?;
    let directory = application_data_directory(app)?.join("exports");
    fs::create_dir_all(&directory).map_err(to_message)?;
    let (extension, content) = match format {
        ExportFormat::Json => ("json", export_json(&attempts).map_err(to_message)?),
        ExportFormat::Csv => ("csv", export_csv(&attempts)),
    };
    let path = directory.join(format!("grade-desk-{}.{}", now_timestamp(), extension));
    fs::write(&path, content).map_err(to_message)?;
    Ok(ExportReceipt {
        format: extension.to_owned(),
        path: path.display().to_string(),
        record_count: attempts.len(),
    })
}

pub(crate) fn clear_local_data(app: &AppHandle) -> Result<(), String> {
    let database = application_data_directory(app)?.join(DATABASE_FILE);
    for path in [
        database.clone(),
        PathBuf::from(format!("{}-wal", database.display())),
        PathBuf::from(format!("{}-shm", database.display())),
    ] {
        if path.exists() {
            fs::remove_file(path).map_err(to_message)?;
        }
    }
    Ok(())
}

pub(crate) fn import_jwxt_grades(
    app: &AppHandle,
    grades: Vec<RemoteGrade>,
) -> Result<ArchiveResult, String> {
    let mut connection = initialized_database(app)?;
    let transaction = connection.transaction().map_err(to_message)?;
    let is_demo: bool = transaction
        .query_row(
            "SELECT display_name = '示例同学' FROM profiles WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if is_demo {
        transaction.execute_batch("DELETE FROM sync_runs; DELETE FROM course_attempts; DELETE FROM courses; DELETE FROM terms;").map_err(to_message)?;
        transaction
            .execute(
                "UPDATE profiles SET display_name = '已连接教务学生' WHERE id = 1",
                [],
            )
            .map_err(to_message)?;
    }
    for grade in grades {
        transaction.execute("INSERT OR IGNORE INTO terms (profile_id, academic_year, semester, train_type) VALUES (1, ?1, ?2, '本科')", params![grade.academic_year, grade.semester]).map_err(to_message)?;
        let term_id: i64 = transaction.query_row("SELECT id FROM terms WHERE profile_id = 1 AND academic_year = ?1 AND semester = ?2 AND train_type = '本科'", params![grade.academic_year, grade.semester], |row| row.get(0)).map_err(to_message)?;
        transaction.execute("INSERT INTO courses (profile_id, course_code, name, category) VALUES (1, ?1, ?2, ?3) ON CONFLICT(profile_id, course_code) DO UPDATE SET name = excluded.name, category = excluded.category", params![grade.course_code, grade.course_name, grade.category]).map_err(to_message)?;
        let course_id: i64 = transaction
            .query_row(
                "SELECT id FROM courses WHERE profile_id = 1 AND course_code = ?1",
                params![grade.course_code],
                |row| row.get(0),
            )
            .map_err(to_message)?;
        transaction.execute("INSERT INTO course_attempts (course_id, term_id, class_number, official_grade, numeric_score, score_kind, grade_point, credit, passed, recorded_at) VALUES (?1, ?2, ?3, ?4, NULL, 'official_grade', ?5, ?6, ?7, ?8) ON CONFLICT(term_id, class_number) DO UPDATE SET official_grade=excluded.official_grade, grade_point=excluded.grade_point, credit=excluded.credit, passed=excluded.passed, recorded_at=excluded.recorded_at", params![course_id, term_id, grade.class_number, grade.official_grade, grade.grade_point, grade.credit, grade.passed, now_timestamp()]).map_err(to_message)?;
    }
    transaction.commit().map_err(to_message)?;
    archive_from(&mut connection).map_err(to_message)
}

fn initialized_database(app: &AppHandle) -> Result<Connection, String> {
    let mut connection = open_application_database(app)?;
    migrate(&connection).map_err(to_message)?;
    seed_demo_data(&mut connection).map_err(to_message)?;
    Ok(connection)
}

fn open_application_database(app: &AppHandle) -> Result<Connection, String> {
    let directory = application_data_directory(app)?;
    let connection = Connection::open(directory.join(DATABASE_FILE)).map_err(to_message)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(to_message)?;
    Ok(connection)
}

fn application_data_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(to_message)?;
    fs::create_dir_all(&directory).map_err(to_message)?;
    Ok(directory)
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
        CREATE TABLE IF NOT EXISTS grade_changes (
            id INTEGER PRIMARY KEY,
            attempt_id INTEGER NOT NULL REFERENCES course_attempts(id) ON DELETE CASCADE,
            before_snapshot_id INTEGER REFERENCES grade_snapshots(id) ON DELETE SET NULL,
            after_snapshot_id INTEGER NOT NULL REFERENCES grade_snapshots(id) ON DELETE CASCADE,
            change_type TEXT NOT NULL,
            detected_at TEXT NOT NULL,
            reviewed_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_course_attempts_term ON course_attempts(term_id);
        CREATE INDEX IF NOT EXISTS idx_grade_snapshots_attempt ON grade_snapshots(attempt_id, id DESC);
        PRAGMA user_version = 3;
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
        params!["示例同学", "示例学校", "2026-07-12T00:00:00Z"],
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

    let components = [
        (1, "课堂练习", Some(96.0), Some(20.0)),
        (1, "课程项目", Some(94.0), Some(30.0)),
        (1, "期末考试", Some(91.0), Some(50.0)),
        (2, "平时作业", Some(93.0), Some(30.0)),
        (2, "期末考试", Some(90.0), Some(70.0)),
    ];
    for (attempt_id, name, score, weight) in components {
        transaction.execute(
            "INSERT INTO score_components (attempt_id, name, score, weight) VALUES (?1, ?2, ?3, ?4)",
            params![attempt_id, name, score, weight],
        )?;
    }

    transaction.execute(
        "INSERT INTO sync_runs (id, profile_id, started_at, finished_at, status, source_version) VALUES (1, 1, ?1, ?1, 'completed', 'seed-v1')",
        params!["2026-07-12T00:00:00Z"],
    )?;
    transaction.execute(
        "INSERT INTO grade_snapshots (sync_run_id, attempt_id, payload_hash, normalized_json, created_at)
         VALUES (1, 1, ?1, ?2, ?3)",
        params!["A||official_grade|4", r#"{"officialGrade":"A","numericScore":null,"scoreKind":"official_grade","gradePoint":4}"#, "2026-07-12T00:00:00Z"],
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
            COALESCE(
                SUM(CASE WHEN a.grade_point IS NOT NULL AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') THEN a.grade_point * a.credit ELSE 0 END)
                / NULLIF(SUM(CASE WHEN a.grade_point IS NOT NULL AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') THEN a.credit ELSE 0 END), 0),
                0
            ),
            COALESCE(
                SUM(CASE WHEN a.grade_point IS NOT NULL AND TRIM(c.category) IN ('专业必修', '专业选修', '公共必修', '专必', '专选', '公必') AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') THEN a.grade_point * a.credit ELSE 0 END)
                / NULLIF(SUM(CASE WHEN a.grade_point IS NOT NULL AND TRIM(c.category) IN ('专业必修', '专业选修', '公共必修', '专必', '专选', '公必') AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') THEN a.credit ELSE 0 END), 0),
                0
            ),
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
                all_gpa: row.get(4)?,
                professional_gpa: row.get(5)?,
                last_synced_at: row.get(6)?,
            })
        },
    )
}

fn attempts_from(connection: &Connection) -> SqlResult<Vec<CourseAttempt>> {
    let mut statement = connection.prepare(
        "
        SELECT a.id, c.name, c.course_code, c.category, a.official_grade, a.numeric_score,
               a.score_kind, a.grade_point, a.credit, a.passed,
               t.academic_year || ' 第' || t.semester || '学期'
        FROM course_attempts a
        JOIN courses c ON c.id = a.course_id
        JOIN terms t ON t.id = a.term_id
        ORDER BY t.academic_year DESC, t.semester DESC, c.course_code
        ",
    )?;
    let rows = statement.query_map([], attempt_from_row)?;
    rows.collect()
}

fn course_detail_from(connection: &Connection, attempt_id: i64) -> SqlResult<CourseDetail> {
    let mut statement = connection.prepare(
        "
        SELECT a.id, c.name, c.course_code, c.category, a.official_grade, a.numeric_score,
               a.score_kind, a.grade_point, a.credit, a.passed,
               t.academic_year || ' 第' || t.semester || '学期', a.class_number
        FROM course_attempts a
        JOIN courses c ON c.id = a.course_id
        JOIN terms t ON t.id = a.term_id
        WHERE a.id = ?1
        ",
    )?;
    let (attempt, term, class_number) = statement.query_row(params![attempt_id], |row| {
        Ok((attempt_from_row(row)?, row.get(10)?, row.get(11)?))
    })?;

    let mut components_statement = connection.prepare(
        "SELECT name, score, weight FROM score_components WHERE attempt_id = ?1 ORDER BY id",
    )?;
    let components = components_statement
        .query_map(params![attempt_id], |row| {
            Ok(ScoreComponent {
                name: row.get(0)?,
                score: row.get(1)?,
                weight: row.get(2)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

    Ok(CourseDetail {
        attempt,
        term,
        class_number,
        components,
    })
}

fn attempt_from_row(row: &rusqlite::Row<'_>) -> SqlResult<CourseAttempt> {
    Ok(CourseAttempt {
        id: row.get(0)?,
        course_name: row.get(1)?,
        course_code: row.get(2)?,
        category: row.get(3)?,
        official_grade: row.get(4)?,
        numeric_score: row.get(5)?,
        score_kind: row.get(6)?,
        grade_point: row.get(7)?,
        credit: row.get(8)?,
        passed: row.get::<_, i64>(9)? == 1,
        term: row.get(10).unwrap_or_else(|_| String::new()),
    })
}

fn analysis_overview_from(connection: &Connection) -> SqlResult<AnalysisOverview> {
    let mut trend_statement = connection.prepare(
        "SELECT t.academic_year || ' 第' || t.semester || '学期', SUM(CASE WHEN a.grade_point IS NOT NULL AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') THEN a.grade_point * a.credit ELSE 0 END) / NULLIF(SUM(CASE WHEN a.grade_point IS NOT NULL AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') THEN a.credit ELSE 0 END), 0), SUM(CASE WHEN a.passed = 1 THEN a.credit ELSE 0 END), COUNT(a.id) FROM terms t JOIN course_attempts a ON a.term_id = t.id WHERE t.profile_id = 1 GROUP BY t.id ORDER BY t.academic_year, t.semester",
    )?;
    let trends = trend_statement.query_map([], |row| Ok(TermTrend { term: row.get(0)?, gpa: row.get(1)?, earned_credits: row.get(2)?, course_count: row.get(3)? }))?.collect::<SqlResult<Vec<_>>>()?;
    let overall_gpa: Option<f64> = connection.query_row("SELECT SUM(grade_point * credit) / NULLIF(SUM(credit), 0) FROM course_attempts WHERE grade_point IS NOT NULL AND UPPER(TRIM(COALESCE(official_grade, ''))) NOT IN ('P', 'NP')", [], |row| row.get(0))?;
    let contributions = if let Some(gpa) = overall_gpa {
        let mut statement = connection.prepare("SELECT a.id, c.name, c.course_code, a.credit, a.grade_point FROM course_attempts a JOIN courses c ON c.id = a.course_id WHERE a.grade_point IS NOT NULL AND UPPER(TRIM(COALESCE(a.official_grade, ''))) NOT IN ('P', 'NP') ORDER BY ABS(a.credit * (a.grade_point - ?1)) DESC, c.course_code")?;
        let rows = statement.query_map(params![gpa], |row| { let credit: f64 = row.get(3)?; let grade_point: f64 = row.get(4)?; Ok(CourseContribution { attempt_id: row.get(0)?, course_name: row.get(1)?, course_code: row.get(2)?, credit, grade_point, contribution: credit * (grade_point - gpa) }) })?.collect::<SqlResult<Vec<_>>>()?;
        rows
    } else { Vec::new() };
    let distribution = [("90–100", 90_i64, 101_i64), ("80–89", 80, 90), ("70–79", 70, 80), ("60–69", 60, 70), ("0–59", 0, 60)].into_iter().map(|(label, low, high)| {
        let count = connection.query_row("SELECT COUNT(*) FROM course_attempts WHERE score_kind = 'official_numeric' AND numeric_score >= ?1 AND numeric_score < ?2", params![low, high], |row| row.get(0))?;
        Ok(ScoreDistributionBin { label: label.to_owned(), count })
    }).collect::<SqlResult<Vec<_>>>()?;
    let (numeric_count, grade_only_count, pass_fail_count, unavailable_count) = connection.query_row("SELECT SUM(CASE WHEN score_kind = 'official_numeric' THEN 1 ELSE 0 END), SUM(CASE WHEN score_kind = 'official_grade' AND UPPER(TRIM(COALESCE(official_grade, ''))) NOT IN ('P', 'NP') THEN 1 ELSE 0 END), SUM(CASE WHEN UPPER(TRIM(COALESCE(official_grade, ''))) IN ('P', 'NP') THEN 1 ELSE 0 END), SUM(CASE WHEN score_kind IN ('derived', 'unavailable') THEN 1 ELSE 0 END) FROM course_attempts", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?;
    let as_of = connection.query_row("SELECT COALESCE(MAX(finished_at), '') FROM sync_runs WHERE profile_id = 1 AND status = 'completed'", [], |row| row.get(0))?;
    Ok(AnalysisOverview { trends, contributions, distribution, data_quality: AnalysisDataQuality { numeric_count, grade_only_count, pass_fail_count, unavailable_count }, as_of })
}

#[derive(Debug)]
struct SnapshotInput {
    attempt_id: i64,
    official_grade: Option<String>,
    numeric_score: Option<f64>,
    score_kind: String,
    grade_point: Option<f64>,
}

fn snapshot_inputs_from(connection: &Connection) -> SqlResult<Vec<SnapshotInput>> {
    let mut statement = connection.prepare(
        "SELECT id, official_grade, numeric_score, score_kind, grade_point FROM course_attempts ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SnapshotInput {
            attempt_id: row.get(0)?,
            official_grade: row.get(1)?,
            numeric_score: row.get(2)?,
            score_kind: row.get(3)?,
            grade_point: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn archive_from(connection: &mut Connection) -> SqlResult<ArchiveResult> {
    let inputs = snapshot_inputs_from(connection)?;
    let finished_at = now_timestamp();
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO sync_runs (profile_id, started_at, finished_at, status, source_version) VALUES (1, ?1, ?1, 'completed', 'local-archive-v1')",
        params![finished_at],
    )?;
    let sync_run_id = transaction.last_insert_rowid();
    let mut changes_detected = 0;
    let has_baseline: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM grade_snapshots)", [], |row| row.get(0))?;
    let incomplete_seed_baseline: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM sync_runs WHERE source_version = 'seed-v1') AND (SELECT COUNT(*) FROM grade_snapshots) < ?1",
        params![inputs.len() as i64],
        |row| row.get(0),
    )?;

    for input in &inputs {
        let (payload_hash, normalized_json) = snapshot_payload(input);
        let previous: Option<(i64, String)> = transaction
            .query_row(
                "SELECT id, payload_hash FROM grade_snapshots WHERE attempt_id = ?1 ORDER BY id DESC LIMIT 1",
                params![input.attempt_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        transaction.execute(
            "INSERT INTO grade_snapshots (sync_run_id, attempt_id, payload_hash, normalized_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sync_run_id, input.attempt_id, payload_hash, normalized_json, finished_at],
        )?;
        let after_snapshot_id = transaction.last_insert_rowid();
        if let Some((before_snapshot_id, before_hash)) = previous {
            if before_hash != payload_hash {
                transaction.execute(
                    "INSERT INTO grade_changes (attempt_id, before_snapshot_id, after_snapshot_id, change_type, detected_at) VALUES (?1, ?2, ?3, 'grade_updated', ?4)",
                    params![input.attempt_id, before_snapshot_id, after_snapshot_id, finished_at],
                )?;
                changes_detected += 1;
            }
        } else if has_baseline && !incomplete_seed_baseline {
            transaction.execute(
                "INSERT INTO grade_changes (attempt_id, before_snapshot_id, after_snapshot_id, change_type, detected_at) VALUES (?1, NULL, ?2, 'course_added', ?3)",
                params![input.attempt_id, after_snapshot_id, finished_at],
            )?;
            changes_detected += 1;
        }
    }
    transaction.commit()?;
    Ok(ArchiveResult {
        sync_run_id,
        snapshot_count: inputs.len(),
        changes_detected,
        finished_at,
    })
}

fn snapshot_payload(input: &SnapshotInput) -> (String, String) {
    let number = input
        .numeric_score
        .map(|value| value.to_string())
        .unwrap_or_default();
    let point = input
        .grade_point
        .map(|value| value.to_string())
        .unwrap_or_default();
    let grade = input.official_grade.clone().unwrap_or_default();
    let payload_hash = format!("{grade}|{number}|{}|{point}", input.score_kind);
    let normalized_json = serde_json::json!({
        "officialGrade": input.official_grade,
        "numericScore": input.numeric_score,
        "scoreKind": input.score_kind,
        "gradePoint": input.grade_point,
    })
    .to_string();
    (payload_hash, normalized_json)
}

fn sync_runs_from(connection: &Connection) -> SqlResult<Vec<SyncRun>> {
    let mut statement = connection.prepare(
        "
        SELECT r.id, COALESCE(r.finished_at, r.started_at), r.source_version,
               COUNT(DISTINCT s.id), COUNT(DISTINCT c.id)
        FROM sync_runs r
        LEFT JOIN grade_snapshots s ON s.sync_run_id = r.id
        LEFT JOIN grade_changes c ON c.after_snapshot_id = s.id
        GROUP BY r.id
        ORDER BY r.id DESC
        ",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SyncRun {
            id: row.get(0)?,
            finished_at: row.get(1)?,
            source_version: row.get(2)?,
            snapshot_count: row.get(3)?,
            change_count: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn pending_changes_from(connection: &Connection) -> SqlResult<Vec<ChangeRecord>> {
    let mut statement = connection.prepare(
        "
        SELECT g.id, c.name, c.course_code, g.detected_at, g.change_type
        FROM grade_changes g
        JOIN course_attempts a ON a.id = g.attempt_id
        JOIN courses c ON c.id = a.course_id
        WHERE g.reviewed_at IS NULL
        ORDER BY g.id DESC
        ",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ChangeRecord {
            id: row.get(0)?,
            course_name: row.get(1)?,
            course_code: row.get(2)?,
            detected_at: row.get(3)?,
            change_type: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn export_json(attempts: &[CourseAttempt]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(attempts)
}

fn export_csv(attempts: &[CourseAttempt]) -> String {
    let mut rows =
        vec!["课程代码,课程名称,课程类别,官方成绩,数值成绩,成绩来源,绩点,学分,是否通过".to_owned()];
    rows.extend(attempts.iter().map(|attempt| {
        [
            csv_field(&attempt.course_code),
            csv_field(&attempt.course_name),
            csv_field(&attempt.category),
            csv_field(attempt.official_grade.as_deref().unwrap_or("")),
            attempt
                .numeric_score
                .map(|value| value.to_string())
                .unwrap_or_default(),
            csv_field(&attempt.score_kind),
            attempt
                .grade_point
                .map(|value| value.to_string())
                .unwrap_or_default(),
            attempt.credit.to_string(),
            attempt.passed.to_string(),
        ]
        .join(",")
    }));
    rows.join("\n")
}

fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn now_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
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
        assert!((dashboard.all_gpa - 3.78).abs() < 0.01);
        assert!((dashboard.professional_gpa - 3.74).abs() < 0.01);

        connection
            .execute(
                "INSERT INTO courses (id, profile_id, course_code, name, category) VALUES (5, 1, 'PN101', '通过制课程', '专业必修')",
                [],
            )
            .expect("add pass-no-pass course");
        connection
            .execute(
                "INSERT INTO course_attempts (id, course_id, term_id, class_number, official_grade, numeric_score, score_kind, grade_point, credit, passed, recorded_at) VALUES (5, 5, 1, 'PN101-01', 'P', NULL, 'official_grade', 4.0, 3.0, 1, '2026-07-12T00:00:00Z')",
                [],
            )
            .expect("add pass-no-pass attempt");
        let without_pass_no_pass = dashboard_from(&connection).expect("recalculate dashboard");
        assert!((without_pass_no_pass.all_gpa - 3.78).abs() < 0.01);
        assert!((without_pass_no_pass.professional_gpa - 3.74).abs() < 0.01);

        let attempts = attempts_from(&connection).expect("load attempts");
        assert_eq!(attempts.len(), 5);
        assert_eq!(attempts[0].term, "2025-2026 第1学期");
        let detail = course_detail_from(&connection, 1).expect("load course detail");
        assert_eq!(detail.components.len(), 3);

        let initial_archive = archive_from(&mut connection).expect("archive current data");
        assert_eq!(initial_archive.snapshot_count, 5);
        assert_eq!(initial_archive.changes_detected, 0);

        connection
            .execute(
                "UPDATE course_attempts SET official_grade = 'B' WHERE id = 1",
                [],
            )
            .expect("change demo grade");
        let changed_archive = archive_from(&mut connection).expect("archive changed data");
        assert_eq!(changed_archive.changes_detected, 1);
        assert_eq!(
            pending_changes_from(&connection)
                .expect("list changes")
                .len(),
            1
        );
        assert_eq!(export_csv(&attempts).lines().count(), 6);
    }

    #[test]
    fn analysis_and_archive_distinguish_terms_and_new_courses() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&connection).expect("apply schema");
        seed_demo_data(&mut connection).expect("seed data");
        let baseline = archive_from(&mut connection).expect("complete seed baseline");
        assert_eq!(baseline.changes_detected, 0);
        connection.execute("INSERT INTO terms (id, profile_id, academic_year, semester, train_type) VALUES (2, 1, '2024-2025', 2, '本科')", []).expect("add earlier term");
        connection.execute("INSERT INTO courses (id, profile_id, course_code, name, category) VALUES (5, 1, 'NEW101', '新增课程', '专业必修')", []).expect("add course");
        connection.execute("INSERT INTO course_attempts (id, course_id, term_id, class_number, official_grade, numeric_score, score_kind, grade_point, credit, passed, recorded_at) VALUES (5, 5, 2, 'NEW101-01', 'A', 93, 'official_numeric', 4.0, 2.0, 1, '2026-07-13T00:00:00Z')", []).expect("add attempt");
        let archive = archive_from(&mut connection).expect("archive new course");
        assert_eq!(archive.changes_detected, 1);
        assert_eq!(pending_changes_from(&connection).expect("changes")[0].change_type, "course_added");
        let analysis = analysis_overview_from(&connection).expect("analysis");
        assert_eq!(analysis.trends.len(), 2);
        assert_eq!(analysis.distribution.iter().map(|bin| bin.count).sum::<i64>(), 3);
        assert_eq!(analysis.data_quality.grade_only_count, 2);
        let mut statement = connection.prepare("SELECT id, academic_year || ' 第' || semester || '学期' FROM terms WHERE profile_id = 1 ORDER BY academic_year DESC, semester DESC").expect("terms");
        let terms = statement.query_map([], |row| Ok(TermOption { id: row.get(0)?, label: row.get(1)? })).expect("map terms").collect::<SqlResult<Vec<_>>>().expect("collect terms");
        assert_eq!(terms[0].label, "2025-2026 第1学期");
    }

    #[test]
    fn professional_gpa_accepts_jwxt_abbreviated_categories() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&connection).expect("apply schema");
        seed_demo_data(&mut connection).expect("seed data");

        connection
            .execute(
                "INSERT INTO courses (id, profile_id, course_code, name, category) VALUES (5, 1, 'AB101', '缩写专业课', ' 专必 ')",
                [],
            )
            .expect("add abbreviated professional course");
        connection
            .execute(
                "INSERT INTO course_attempts (id, course_id, term_id, class_number, official_grade, numeric_score, score_kind, grade_point, credit, passed, recorded_at) VALUES (5, 5, 1, 'AB101-01', 'A', 91, 'official_numeric', 4.0, 1.0, 1, '2026-07-12T00:00:00Z')",
                [],
            )
            .expect("add abbreviated professional attempt");
        connection
            .execute(
                "INSERT INTO courses (id, profile_id, course_code, name, category) VALUES (6, 1, 'EL101', '缩写公共选修课', '公选')",
                [],
            )
            .expect("add abbreviated elective course");
        connection
            .execute(
                "INSERT INTO course_attempts (id, course_id, term_id, class_number, official_grade, numeric_score, score_kind, grade_point, credit, passed, recorded_at) VALUES (6, 6, 1, 'EL101-01', 'B', 82, 'official_numeric', 3.3, 1.0, 1, '2026-07-12T00:00:00Z')",
                [],
            )
            .expect("add abbreviated elective attempt");

        let dashboard = dashboard_from(&connection).expect("recalculate dashboard");
        assert!((dashboard.all_gpa - (49.1 + 4.0 + 3.3) / 15.0).abs() < 0.001);
        assert!((dashboard.professional_gpa - (41.1 + 4.0) / 12.0).abs() < 0.001);
    }
}
