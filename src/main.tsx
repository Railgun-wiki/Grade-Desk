import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

type AppStatus = {
  name: string;
  version: string;
  storageMode: string;
};

type Dashboard = {
  profileName: string;
  currentTerm: string;
  cumulativeGpa: number;
  earnedCredits: number;
  courseCount: number;
  lastSyncedAt: string;
};

type CourseAttempt = {
  id: number;
  courseName: string;
  courseCode: string;
  category: string;
  officialGrade: string | null;
  numericScore: number | null;
  scoreKind: "official_numeric" | "official_grade" | "derived" | "unavailable";
  gradePoint: number | null;
  credit: number;
  passed: boolean;
};

type ScoreComponent = { name: string; score: number | null; weight: number | null };
type CourseDetail = CourseAttempt & { term: string; classNumber: string | null; components: ScoreComponent[] };
type SyncRun = { id: number; finishedAt: string; sourceVersion: string; snapshotCount: number; changeCount: number };
type ChangeRecord = { id: number; courseName: string; courseCode: string; detectedAt: string; changeType: string };
type ArchiveResult = { syncRunId: number; snapshotCount: number; changesDetected: number; finishedAt: string };
type ExportReceipt = { format: string; path: string; recordCount: number };
type JwxtStatus = { connected: boolean; message: string };
type GradeQueryResult = { courseCount: number; trainType: string };

const previewDashboard: Dashboard = {
  profileName: "示例同学", currentTerm: "2025-2026 第1学期", cumulativeGpa: 3.78,
  earnedCredits: 13, courseCount: 4, lastSyncedAt: "2026-07-12T00:00:00Z",
};
const previewAttempts: CourseAttempt[] = [
  { id: 1, courseName: "程序设计基础", courseCode: "CS101", category: "专业必修", officialGrade: "A", numericScore: null, scoreKind: "official_grade", gradePoint: 4, credit: 4, passed: true },
  { id: 2, courseName: "高等数学", courseCode: "MA101", category: "公共必修", officialGrade: "A-", numericScore: 91, scoreKind: "official_numeric", gradePoint: 3.7, credit: 5, passed: true },
  { id: 3, courseName: "学术英语", courseCode: "EN101", category: "公共必修", officialGrade: "B+", numericScore: null, scoreKind: "official_grade", gradePoint: 3.3, credit: 2, passed: true },
  { id: 4, courseName: "科学与社会", courseCode: "GE101", category: "通识选修", officialGrade: "A", numericScore: 95, scoreKind: "official_numeric", gradePoint: 4, credit: 2, passed: true },
];

function gradeLabel(course: CourseAttempt) {
  const score = course.numericScore === null ? "" : ` · ${course.numericScore}`;
  return `${course.officialGrade ?? "未出分"}${score}`;
}

function scoreSource(course: CourseAttempt) {
  return course.scoreKind === "official_numeric" ? "教务数值" : "官方等级";
}

function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [dashboard, setDashboard] = useState<Dashboard>(previewDashboard);
  const [attempts, setAttempts] = useState<CourseAttempt[]>(previewAttempts);
  const [activeView, setActiveView] = useState<"overview" | "transcript" | "archive" | "connection">("overview");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [detail, setDetail] = useState<CourseDetail | null>(null);
  const [query, setQuery] = useState("");
  const [syncRuns, setSyncRuns] = useState<SyncRun[]>([]);
  const [changes, setChanges] = useState<ChangeRecord[]>([]);
  const [notice, setNotice] = useState("");
  const [jwxt, setJwxt] = useState<JwxtStatus>({ connected: false, message: "正在检查教务会话…" });

  useEffect(() => {
    void invoke<AppStatus>("application_status").then(setStatus).catch(() => {
      setStatus({ name: "Grade Desk", version: "Web preview", storageMode: "local-only" });
    });
  }, []);
  useEffect(() => { void invoke<JwxtStatus>("jwxt_status").then(setJwxt).catch(() => undefined); }, []);

  const refreshArchive = () => {
    void Promise.all([invoke<SyncRun[]>("list_sync_runs"), invoke<ChangeRecord[]>("list_pending_changes")])
      .then(([nextRuns, nextChanges]) => { setSyncRuns(nextRuns); setChanges(nextChanges); })
      .catch(() => undefined);
  };

  useEffect(() => { refreshArchive(); }, []);

  const createArchive = async () => {
    try {
      const result = await invoke<ArchiveResult>("archive_current_data");
      setNotice(`已创建 ${result.snapshotCount} 份本地快照；发现 ${result.changesDetected} 项变更。`);
      refreshArchive();
    } catch { setNotice("无法创建本地快照。请稍后重试。"); }
  };

  const exportData = async (format: "json" | "csv") => {
    try {
      const receipt = await invoke<ExportReceipt>("export_grade_data", { format });
      setNotice(`已导出 ${receipt.recordCount} 条记录至 ${receipt.path}`);
    } catch { setNotice("导出失败。请稍后重试。"); }
  };

  const reviewChanges = async () => {
    try {
      const reviewed = await invoke<number>("review_pending_changes");
      setNotice(`已审阅 ${reviewed} 项变更。`); refreshArchive();
    } catch { setNotice("无法更新审阅状态。请稍后重试。"); }
  };

  const clearData = async () => {
    if (!window.confirm("确定要清除本机保存的成绩档案吗？此操作不会影响学校教务系统，且无法撤销。")) return;
    try {
      await invoke("clear_local_data");
      setNotice("本地数据库已清除。重新打开应用时会创建匿名示例档案。");
      setSyncRuns([]); setChanges([]); setSelectedId(null);
    } catch { setNotice("无法清除本地数据。请稍后重试。"); }
  };
  const startJwxtLogin = async () => { try { await invoke("start_jwxt_login"); setNotice("已打开受控教务登录窗口；完成登录后返回此处验证会话。"); } catch { setNotice("无法打开教务登录窗口。"); } };
  const verifyJwxt = async () => { try { const saved = await invoke<JwxtStatus>("save_jwxt_session"); setJwxt(saved); const result = await invoke<GradeQueryResult>("verify_jwxt_session"); setJwxt({ connected: true, message: `会话有效，可读取 ${result.courseCount} 门课程（${result.trainType}）。` }); } catch (error) { setJwxt({ connected: false, message: String(error) }); } };
  const syncJwxt = async () => { try { const result = await invoke<GradeQueryResult>("sync_jwxt_grades"); setJwxt({ connected: true, message: `已同步 ${result.courseCount} 门官方课程。` }); setNotice("真实教务成绩已写入本地档案。"); } catch (error) { setJwxt({ connected: false, message: String(error) }); } };

  useEffect(() => {
    void Promise.all([invoke<Dashboard>("get_dashboard"), invoke<CourseAttempt[]>("list_course_attempts")])
      .then(([nextDashboard, nextAttempts]) => { setDashboard(nextDashboard); setAttempts(nextAttempts); })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    if (selectedId === null) { setDetail(null); return; }
    void invoke<CourseDetail>("get_course_detail", { attemptId: selectedId }).then(setDetail).catch(() => {
      const attempt = attempts.find((item) => item.id === selectedId);
      if (attempt) setDetail({ ...attempt, term: dashboard.currentTerm, classNumber: null, components: [] });
    });
  }, [attempts, dashboard.currentTerm, selectedId]);

  const filteredAttempts = useMemo(() => attempts.filter((item) =>
    `${item.courseName}${item.courseCode}${item.category}`.toLowerCase().includes(query.trim().toLowerCase())), [attempts, query]);

  return (
    <div className="app-shell">
      <header className="global-nav"><strong>Grade Desk</strong><span>{status ? status.storageMode : "本地优先"}</span></header>
      <header className="context-nav"><span>成绩</span><span className="term-chip">{dashboard.currentTerm}</span><button className="primary-button" type="button" onClick={() => void createArchive()}>创建快照</button></header>
      <aside className="sidebar" aria-label="主导航">
        <button className={activeView === "overview" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("overview")} type="button">概览</button>
        <button className={activeView === "transcript" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("transcript")} type="button">成绩单</button>
        <button className="nav-item" type="button" disabled>分析 <span>即将推出</span></button>
        <button className={activeView === "archive" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("archive")} type="button">归档</button>
        <button className={activeView === "connection" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("connection")} type="button">连接教务</button>
      </aside>
      <main className="content" id="main-content">
        {notice && <p className="notice" role="status">{notice}</p>}
        {activeView === "overview" && <Overview dashboard={dashboard} attempts={attempts} onTranscript={() => setActiveView("transcript")} />}
        {activeView === "transcript" && <Transcript attempts={filteredAttempts} query={query} onQuery={setQuery} onSelect={setSelectedId} />}
        {activeView === "archive" && <Archive runs={syncRuns} changes={changes} onReview={() => void reviewChanges()} onExport={exportData} onClear={() => void clearData()} />}
        {activeView === "connection" && <Connection status={jwxt} onLogin={() => void startJwxtLogin()} onVerify={() => void verifyJwxt()} onSync={() => void syncJwxt()} />}
      </main>
      {detail && <CoursePanel detail={detail} onClose={() => setSelectedId(null)} />}
    </div>
  );
}

function Overview({ dashboard, attempts, onTranscript }: { dashboard: Dashboard; attempts: CourseAttempt[]; onTranscript: () => void }) {
  return <>
    <section className="welcome" aria-labelledby="overview-title">
      <p className="eyebrow">个人成绩档案</p><h1 id="overview-title">你好，{dashboard.profileName}。</h1>
      <p>这是一份只属于你的本地学业记录。</p>
    </section>
    <section className="metric-grid" aria-label="成绩摘要">
      <Metric label="累计 GPA" value={dashboard.cumulativeGpa.toFixed(2)} note="本地计算" />
      <Metric label="已获学分" value={dashboard.earnedCredits.toFixed(0)} note="全部通过课程" />
      <Metric label="已归档课程" value={String(dashboard.courseCount)} note="当前学期" />
    </section>
    <section className="two-column">
      <div className="section-card"><div className="section-heading"><div><p className="eyebrow">本学期</p><h2>课程表现</h2></div><button className="text-button" onClick={onTranscript} type="button">查看成绩单</button></div>
        <div className="course-summary">{attempts.map((course) => <div key={course.id}><span>{course.courseName}</span><strong>{gradeLabel(course)}</strong></div>)}</div>
      </div>
      <div className="dark-card"><p className="eyebrow">数据状态</p><h2>安全地留在这里。</h2><p>上次本地归档于 {dashboard.lastSyncedAt.slice(0, 10)}。目前展示的是示例档案，尚未连接教务系统。</p><span className="dark-link">本地存储 · 未连接账号</span></div>
    </section>
  </>;
}

function Metric({ label, value, note }: { label: string; value: string; note: string }) { return <article className="metric"><p>{label}</p><strong>{value}</strong><span>{note}</span></article>; }

function Transcript({ attempts, query, onQuery, onSelect }: { attempts: CourseAttempt[]; query: string; onQuery: (value: string) => void; onSelect: (id: number) => void }) {
  return <section aria-labelledby="transcript-title"><div className="page-heading"><div><p className="eyebrow">成绩单</p><h1 id="transcript-title">所有课程</h1></div><label className="search"><span>搜索</span><input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="课程名称或代码" /></label></div>
    <div className="table-card"><div className="table-header" aria-hidden="true"><span>课程</span><span>类别</span><span>学分</span><span>成绩</span><span>绩点</span></div>
      {attempts.map((course) => <button key={course.id} className="course-row" onClick={() => onSelect(course.id)} type="button"><span><strong>{course.courseName}</strong><small>{course.courseCode} · {scoreSource(course)}</small></span><span>{course.category}</span><span>{course.credit.toFixed(1)}</span><span><b>{gradeLabel(course)}</b><small>{course.passed ? "已通过" : "未通过"}</small></span><span>{course.gradePoint?.toFixed(1) ?? "—"}</span></button>)}
      {attempts.length === 0 && <p className="empty-state">没有匹配的课程。</p>}
    </div>
  </section>;
}

function CoursePanel({ detail, onClose }: { detail: CourseDetail; onClose: () => void }) {
  return <aside className="detail-panel" aria-labelledby="course-title"><button className="close-button" type="button" onClick={onClose} aria-label="关闭课程详情">×</button><p className="eyebrow">{detail.term}</p><h2 id="course-title">{detail.courseName}</h2><p className="course-code">{detail.courseCode} · {detail.category}</p><div className="grade-hero"><span>{scoreSource(detail)}</span><strong>{gradeLabel(detail)}</strong><small>{detail.gradePoint?.toFixed(1) ?? "—"} 绩点 · {detail.credit.toFixed(1)} 学分</small></div><section><h3>成绩构成</h3>{detail.components.length > 0 ? <div className="component-list">{detail.components.map((component) => <div key={component.name}><span>{component.name}</span><span>{component.score ?? "—"} · {component.weight ?? "—"}%</span></div>)}</div> : <p className="muted">教务未提供成绩构成。</p>}</section><section><h3>修读信息</h3><dl><div><dt>教学班</dt><dd>{detail.classNumber ?? "教务未提供"}</dd></div><div><dt>状态</dt><dd>{detail.passed ? "已通过" : "未通过"}</dd></div></dl></section></aside>;
}

function Archive({ runs, changes, onReview, onExport, onClear }: { runs: SyncRun[]; changes: ChangeRecord[]; onReview: () => void; onExport: (format: "json" | "csv") => void; onClear: () => void }) {
  return <section aria-labelledby="archive-title"><div className="page-heading"><div><p className="eyebrow">归档</p><h1 id="archive-title">本地历史</h1></div><div className="archive-actions"><button className="secondary-button" type="button" onClick={() => onExport("csv")}>导出 CSV</button><button className="secondary-button" type="button" onClick={() => onExport("json")}>导出 JSON</button></div></div>
    <div className="archive-grid"><section className="section-card"><div className="section-heading"><div><p className="eyebrow">待审阅</p><h2>检测到的变更</h2></div>{changes.length > 0 && <button className="text-button" type="button" onClick={onReview}>全部标记已审阅</button>}</div>
      {changes.length > 0 ? <div className="change-list">{changes.map((change) => <div key={change.id}><span><strong>{change.courseName}</strong><small>{change.courseCode} · {change.changeType}</small></span><time>{change.detectedAt}</time></div>)}</div> : <p className="muted padded">当前没有待审阅的成绩变更。</p>}
    </section><section className="section-card"><p className="eyebrow">隐私</p><h2>管理本机数据</h2><p className="archive-copy">导出文件会保存到应用数据目录。清除只影响此设备，不会修改教务系统。</p><button className="danger-button" type="button" onClick={onClear}>清除本地档案</button></section></div>
    <section className="table-card archive-table"><div className="section-heading"><div><p className="eyebrow">快照</p><h2>归档记录</h2></div></div>{runs.length > 0 ? runs.map((run) => <div className="run-row" key={run.id}><span><strong>本地快照 #{run.id}</strong><small>{run.sourceVersion}</small></span><span>{run.snapshotCount} 门课程</span><span>{run.changeCount} 项变更</span><time>{run.finishedAt}</time></div>) : <p className="empty-state">还没有本地快照。</p>}</section>
  </section>;
}

function Connection({ status, onLogin, onVerify, onSync }: { status: JwxtStatus; onLogin: () => void; onVerify: () => void; onSync: () => void }) {
  return <section aria-labelledby="connection-title"><div className="page-heading"><div><p className="eyebrow">连接教务</p><h1 id="connection-title">受控登录</h1></div></div><div className="connection-card"><p className="eyebrow">CAS · JWXT</p><h2>{status.connected ? "教务会话已保存" : "在应用内完成统一认证"}</h2><p>{status.message}</p><div className="archive-actions"><button className="primary-button" type="button" onClick={onLogin}>打开教务登录</button><button className="secondary-button" type="button" onClick={onVerify}>验证会话</button><button className="secondary-button" type="button" onClick={onSync}>同步官方成绩</button></div><p className="muted">密码仅在教务登录页面中输入。会话 Cookie 保存在应用数据目录的本地文件，权限仅限当前用户；应用数据库不保存密码或 Cookie。</p></div></section>;
}

createRoot(document.getElementById("root")!).render(<App />);
