import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

type AppStatus = {
  name: string;
  version: string;
  storageMode: string;
  os: string;
};

type Dashboard = {
  profileName: string;
  currentTerm: string;
  allGpa: number;
  professionalGpa: number;
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
  term: string;
};

type ScoreComponent = { name: string; score: number | null; weight: number | null };
type CourseDetail = CourseAttempt & { term: string; classNumber: string | null; components: ScoreComponent[] };
type SyncRun = { id: number; finishedAt: string; sourceVersion: string; snapshotCount: number; changeCount: number };
type ChangeRecord = { id: number; courseName: string; courseCode: string; detectedAt: string; changeType: string };
type ArchiveResult = { syncRunId: number; snapshotCount: number; changesDetected: number; finishedAt: string };
type ExportReceipt = { format: string; path: string; recordCount: number };
type JwxtStatus = { connected: boolean; message: string };
type GradeQueryMethod = "officialList" | "achievementSearch";
type GradeQueryResult = { courseCount: number; trainType: string; method: GradeQueryMethod };
type SessionVerification = { trainType: string };
type NumericProbeResult = { numericScore: number };
type RankSummary = { trainType: string; totalRank: string | null; termRank: string | null; totalStudents: string | null; cumulativeGpa: string | null; termGpa: string | null; earnedCredits: string | null };
type TermOption = { id: number; label: string };
type TermTrend = { term: string; gpa: number | null; earnedCredits: number; courseCount: number };
type CourseContribution = { attemptId: number; courseName: string; courseCode: string; credit: number; gradePoint: number; contribution: number };
type ScoreDistributionBin = { label: string; count: number };
type AnalysisOverview = { trends: TermTrend[]; contributions: CourseContribution[]; distribution: ScoreDistributionBin[]; dataQuality: { numericCount: number; gradeOnlyCount: number; passFailCount: number; unavailableCount: number }; asOf: string };

const previewDashboard: Dashboard = {
  profileName: "示例同学", currentTerm: "2025-2026 第1学期", allGpa: 3.78, professionalGpa: 3.74,
  earnedCredits: 13, courseCount: 4, lastSyncedAt: "2026-07-12T00:00:00Z",
};
const previewAttempts: CourseAttempt[] = [
  { id: 1, courseName: "程序设计基础", courseCode: "CS101", category: "专业必修", officialGrade: "A", numericScore: null, scoreKind: "official_grade", gradePoint: 4, credit: 4, passed: true, term: "2025-2026 第1学期" },
  { id: 2, courseName: "高等数学", courseCode: "MA101", category: "公共必修", officialGrade: "A-", numericScore: 91, scoreKind: "official_numeric", gradePoint: 3.7, credit: 5, passed: true, term: "2025-2026 第1学期" },
  { id: 3, courseName: "学术英语", courseCode: "EN101", category: "公共必修", officialGrade: "B+", numericScore: null, scoreKind: "official_grade", gradePoint: 3.3, credit: 2, passed: true, term: "2025-2026 第1学期" },
  { id: 4, courseName: "科学与社会", courseCode: "GE101", category: "通识选修", officialGrade: "A", numericScore: 95, scoreKind: "official_numeric", gradePoint: 4, credit: 2, passed: true, term: "2025-2026 第1学期" },
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
  const [activeView, setActiveView] = useState<"overview" | "transcript" | "analysis" | "archive" | "connection">("overview");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [detail, setDetail] = useState<CourseDetail | null>(null);
  const [query, setQuery] = useState("");
  const [terms, setTerms] = useState<TermOption[]>([]);
  const [selectedTerm, setSelectedTerm] = useState("");
  const [category, setCategory] = useState("all");
  const [passed, setPassed] = useState("all");
  const [scoreKind, setScoreKind] = useState("all");
  const [minimumPoint, setMinimumPoint] = useState("");
  const [maximumPoint, setMaximumPoint] = useState("");
  const [analysis, setAnalysis] = useState<AnalysisOverview | null>(null);
  const [syncRuns, setSyncRuns] = useState<SyncRun[]>([]);
  const [changes, setChanges] = useState<ChangeRecord[]>([]);
  const [notice, setNotice] = useState("");
  const [jwxt, setJwxt] = useState<JwxtStatus>({ connected: false, message: "正在检查教务会话…" });
  const [queryMethod, setQueryMethod] = useState<GradeQueryMethod>("officialList");
  const [rankSummary, setRankSummary] = useState<RankSummary | null>(null);

  useEffect(() => {
    void invoke<AppStatus>("application_status").then((res) => {
      setStatus(res);
      document.body.className = `os-${res.os}`;
    }).catch(() => {
      setStatus({ name: "Grade Desk", version: "Web preview", storageMode: "local-only", os: "macos" });
      document.body.className = "os-macos";
    });
  }, []);
  useEffect(() => { void invoke<JwxtStatus>("jwxt_status").then(setJwxt).catch(() => undefined); }, []);

  const refreshGrades = () => {
    void Promise.all([invoke<Dashboard>("get_dashboard"), invoke<CourseAttempt[]>("list_course_attempts"), invoke<TermOption[]>("list_terms")])
      .then(([nextDashboard, nextAttempts, nextTerms]) => { setDashboard(nextDashboard); setAttempts(nextAttempts); setTerms(nextTerms); setSelectedTerm((current) => current && nextTerms.some((term) => term.label === current) ? current : nextTerms[0]?.label ?? ""); })
      .catch(() => undefined);
  };

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
  const verifyJwxt = async () => { try { if (!jwxt.connected) setJwxt(await invoke<JwxtStatus>("save_jwxt_session")); const result = await invoke<SessionVerification>("verify_jwxt_session"); setJwxt({ connected: true, message: `认证成功；教务会话有效（${result.trainType}）。成绩查询可稍后单独进行。` }); } catch (error) { setJwxt({ connected: false, message: String(error) }); } };
  const syncJwxt = async () => { try { const result = await invoke<GradeQueryResult>("sync_jwxt_grades", { method: queryMethod }); const method = result.method === "officialList" ? "官方成绩单" : "课程成绩检索"; setJwxt({ connected: true, message: `已通过${method}同步 ${result.courseCount} 门课程。` }); setNotice("真实教务成绩已写入本地档案。"); refreshGrades(); } catch (error) { setJwxt({ connected: true, message: String(error) }); } };
  const probeNumericScore = async (attemptId: number) => {
    const result = await invoke<NumericProbeResult>("probe_jwxt_numeric_score", { attemptId });
    refreshGrades();
    return result.numericScore;
  };
  const queryRankSummary = async () => { try { const result = await invoke<RankSummary>("query_jwxt_rank_summary"); setRankSummary(result); setJwxt({ connected: true, message: "排名统计已更新。" }); } catch (error) { setJwxt({ connected: true, message: String(error) }); } };

  useEffect(() => { refreshGrades(); }, []);
  useEffect(() => { if (activeView === "analysis") void invoke<AnalysisOverview>("get_analysis_overview").then(setAnalysis).catch(() => setAnalysis(null)); }, [activeView, attempts]);

  useEffect(() => {
    if (selectedId === null) { setDetail(null); return; }
    let current = true;
    setDetail(null);
    void invoke<CourseDetail>("get_course_detail", { attemptId: selectedId }).then((nextDetail) => {
      if (current) setDetail(nextDetail);
    }).catch(() => {
      if (!current) return;
      const attempt = attempts.find((item) => item.id === selectedId);
      if (attempt) setDetail({ ...attempt, term: dashboard.currentTerm, classNumber: null, components: [] });
    });
    return () => { current = false; };
  }, [attempts, dashboard.currentTerm, selectedId]);

  const filteredAttempts = useMemo(() => attempts.filter((item) =>
    item.term === selectedTerm && `${item.courseName}${item.courseCode}${item.category}`.toLowerCase().includes(query.trim().toLowerCase()) &&
    (category === "all" || item.category === category) && (passed === "all" || String(item.passed) === passed) &&
    (scoreKind === "all" || item.scoreKind === scoreKind) &&
    (minimumPoint === "" || (item.gradePoint !== null && item.gradePoint >= Number(minimumPoint))) &&
    (maximumPoint === "" || (item.gradePoint !== null && item.gradePoint <= Number(maximumPoint)))), [attempts, selectedTerm, query, category, passed, scoreKind, minimumPoint, maximumPoint]);

  return (
    <div className="app-shell">
      <header className="context-nav" data-tauri-drag-region>
        <span className="window-title" data-tauri-drag-region>成绩</span>
      </header>
      <aside className="sidebar" aria-label="主导航" data-tauri-drag-region>
        <div className="nav-items-group">
          <button className={activeView === "overview" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("overview")} type="button">概览</button>
          <button className={activeView === "transcript" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("transcript")} type="button">成绩单</button>
          <button className={activeView === "analysis" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("analysis")} type="button">分析</button>
          <button className={activeView === "archive" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("archive")} type="button">归档</button>
          <button className={activeView === "connection" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("connection")} type="button">连接教务</button>
        </div>
        <div className="sidebar-footer">
          <strong>Grade Desk</strong>
          <span>{status ? status.storageMode : "本地优先"}</span>
        </div>
      </aside>
      <main className={`content ${activeView === "overview" ? "overview-content" : ""}`} id="main-content">
        {notice && <p className="notice" role="status">{notice}</p>}
        {activeView === "overview" && <Overview dashboard={dashboard} attempts={attempts} onTranscript={() => setActiveView("transcript")} />}
        {activeView === "transcript" && <Transcript attempts={filteredAttempts} query={query} onQuery={setQuery} onSelect={setSelectedId} terms={terms} selectedTerm={selectedTerm} onTerm={setSelectedTerm} category={category} onCategory={setCategory} passed={passed} onPassed={setPassed} scoreKind={scoreKind} onScoreKind={setScoreKind} minimumPoint={minimumPoint} onMinimumPoint={setMinimumPoint} maximumPoint={maximumPoint} onMaximumPoint={setMaximumPoint} />}
        {activeView === "analysis" && <Analysis analysis={analysis} onSelect={setSelectedId} />}
        {activeView === "archive" && <Archive runs={syncRuns} changes={changes} onReview={() => void reviewChanges()} onExport={exportData} onClear={() => void clearData()} onCreateArchive={() => void createArchive()} />}
        {activeView === "connection" && <Connection status={jwxt} method={queryMethod} rankSummary={rankSummary} onMethod={setQueryMethod} onLogin={() => void startJwxtLogin()} onVerify={() => void verifyJwxt()} onSync={() => void syncJwxt()} onRank={() => void queryRankSummary()} />}
      </main>
      {detail && <CoursePanel key={detail.id} detail={detail} onClose={() => setSelectedId(null)} onProbe={probeNumericScore} />}
    </div>
  );
}

function Overview({ dashboard, attempts, onTranscript }: { dashboard: Dashboard; attempts: CourseAttempt[]; onTranscript: () => void }) {
  return <section className="overview-page" aria-labelledby="overview-title">
    <section className="welcome" aria-labelledby="overview-title">
      <p className="eyebrow">个人成绩档案</p><h1 id="overview-title">你好，{dashboard.profileName}。</h1>
      <p>这是一份只属于你的本地学业记录。</p>
    </section>
    <section className="metric-grid" aria-label="成绩摘要">
      <Metric label="全部 GPA" value={dashboard.allGpa.toFixed(2)} note="排除 P / NP" />
      <Metric label="专业课 GPA" value={dashboard.professionalGpa.toFixed(2)} note="专必 · 专选 · 公必" />
      <Metric label="已获学分" value={dashboard.earnedCredits.toFixed(0)} note="全部通过课程" />
    </section>
    <section className="two-column">
      <div className="section-card"><div className="section-heading"><div><p className="eyebrow">本学期</p><h2>课程表现</h2></div><button className="text-button" onClick={onTranscript} type="button">查看成绩单</button></div>
        <div className="course-summary">{attempts.map((course) => <div key={course.id}><span>{course.courseName}</span><strong>{gradeLabel(course)}</strong></div>)}</div>
      </div>
      <div className="dark-card"><p className="eyebrow">数据状态</p><h2>安全地留在这里。</h2><p>上次本地归档于 {dashboard.lastSyncedAt.slice(0, 10)}。目前展示的是示例档案，尚未连接教务系统。</p><span className="dark-link">本地存储 · 未连接账号</span></div>
    </section>
  </section>;
}

function Metric({ label, value, note }: { label: string; value: string; note: string }) { return <article className="metric"><p>{label}</p><strong>{value}</strong><span>{note}</span></article>; }

function Transcript({ attempts, query, onQuery, onSelect, terms, selectedTerm, onTerm, category, onCategory, passed, onPassed, scoreKind, onScoreKind, minimumPoint, onMinimumPoint, maximumPoint, onMaximumPoint }: { attempts: CourseAttempt[]; query: string; onQuery: (value: string) => void; onSelect: (id: number) => void; terms: TermOption[]; selectedTerm: string; onTerm: (value: string) => void; category: string; onCategory: (value: string) => void; passed: string; onPassed: (value: string) => void; scoreKind: string; onScoreKind: (value: string) => void; minimumPoint: string; onMinimumPoint: (value: string) => void; maximumPoint: string; onMaximumPoint: (value: string) => void }) {
  const categories = [...new Set(attempts.map((attempt) => attempt.category))];
  return <section aria-labelledby="transcript-title"><div className="page-heading"><div><p className="eyebrow">成绩单</p><h1 id="transcript-title">{selectedTerm || "所有课程"}</h1></div><label className="search"><span>搜索</span><input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="课程名称或代码" /></label></div>
    <div className="filter-bar" aria-label="成绩单筛选"><label>学期<select value={selectedTerm} onChange={(event) => onTerm(event.target.value)}>{terms.map((term) => <option key={term.id} value={term.label}>{term.label}</option>)}</select></label><label>类别<select value={category} onChange={(event) => onCategory(event.target.value)}><option value="all">全部</option>{categories.map((value) => <option key={value} value={value}>{value}</option>)}</select></label><label>状态<select value={passed} onChange={(event) => onPassed(event.target.value)}><option value="all">全部</option><option value="true">已通过</option><option value="false">未通过</option></select></label><label>成绩来源<select value={scoreKind} onChange={(event) => onScoreKind(event.target.value)}><option value="all">全部</option><option value="official_numeric">教务数值</option><option value="official_grade">官方等级</option><option value="derived">本地计算</option><option value="unavailable">未提供</option></select></label><label>最低绩点<input inputMode="decimal" min="0" max="4" value={minimumPoint} onChange={(event) => onMinimumPoint(event.target.value)} placeholder="0.0" /></label><label>最高绩点<input inputMode="decimal" min="0" max="4" value={maximumPoint} onChange={(event) => onMaximumPoint(event.target.value)} placeholder="4.0" /></label></div>
    <div className="table-card"><div className="table-header" aria-hidden="true"><span>课程</span><span>类别</span><span>学分</span><span>成绩</span><span>绩点</span></div>
      {attempts.map((course) => <button key={course.id} className="course-row" onClick={() => onSelect(course.id)} type="button"><span><strong>{course.courseName}</strong><small>{course.courseCode} · {scoreSource(course)}</small></span><span>{course.category}</span><span>{course.credit.toFixed(1)}</span><span><b>{gradeLabel(course)}</b><small>{course.passed ? "已通过" : "未通过"}</small></span><span>{course.gradePoint?.toFixed(1) ?? "—"}</span></button>)}
      {attempts.length === 0 && <p className="empty-state">没有匹配的课程。</p>}
    </div>
  </section>;
}

function Analysis({ analysis, onSelect }: { analysis: AnalysisOverview | null; onSelect: (id: number) => void }) {
  if (!analysis) return <section><div className="page-heading"><div><p className="eyebrow">分析</p><h1>本地学业分析</h1></div></div><p className="empty-state">正在读取本地分析数据。</p></section>;
  const maxDistribution = Math.max(1, ...analysis.distribution.map((item) => item.count));
  return <section aria-labelledby="analysis-title"><div className="page-heading"><div><p className="eyebrow">分析</p><h1 id="analysis-title">本地学业分析</h1><p className="muted">仅基于本机档案{analysis.asOf ? ` · 数据截至 ${analysis.asOf.slice(0, 10)}` : ""}</p></div></div>
    <div className="analysis-grid"><section className="section-card"><p className="eyebrow">学期趋势</p><h2>GPA 与已获学分</h2>{analysis.trends.length ? <div className="trend-list">{analysis.trends.map((trend) => <div key={trend.term}><span>{trend.term}</span><strong>{trend.gpa === null ? "GPA 不可计算" : `GPA ${trend.gpa.toFixed(2)}`}</strong><small>{trend.earnedCredits.toFixed(1)} 已获学分 · {trend.courseCount} 门课程</small></div>)}</div> : <p className="muted">暂无可按学期比较的数据。</p>}</section>
      <section className="section-card"><p className="eyebrow">教务数值</p><h2>数值成绩分布</h2><div className="distribution">{analysis.distribution.map((bin) => <div key={bin.label}><span>{bin.label}</span><i><b style={{ width: `${bin.count / maxDistribution * 100}%` }} /></i><strong>{bin.count}</strong></div>)}</div><p className="muted">只统计已验证的教务数值，不转换官方等级。</p></section></div>
    <section className="table-card contribution-card"><div className="section-heading"><div><p className="eyebrow">课程贡献</p><h2>相对累计 GPA 的影响</h2></div></div>{analysis.contributions.map((course) => <button className="contribution-row" type="button" key={course.attemptId} onClick={() => onSelect(course.attemptId)}><span><strong>{course.courseName}</strong><small>{course.courseCode} · {course.credit.toFixed(1)} 学分 · {course.gradePoint.toFixed(1)} 绩点</small></span><b className={course.contribution >= 0 ? "positive" : "negative"}>{course.contribution >= 0 ? "+" : ""}{course.contribution.toFixed(2)}</b></button>)}{analysis.contributions.length === 0 && <p className="empty-state">没有足够的绩点记录用于贡献分析。</p>}</section>
    <p className="analysis-note">数据覆盖：{analysis.dataQuality.numericCount} 门教务数值、{analysis.dataQuality.gradeOnlyCount} 门官方等级、{analysis.dataQuality.passFailCount} 门 P/NP、{analysis.dataQuality.unavailableCount} 门未提供或本地计算。P/NP 不计入 GPA。</p>
  </section>;
}

function CoursePanel({ detail, onClose, onProbe }: { detail: CourseDetail; onClose: () => void; onProbe: (attemptId: number) => Promise<number> }) {
  const [probeState, setProbeState] = useState<"idle" | "confirm" | "running" | "done" | "error">("idle");
  const [probeMessage, setProbeMessage] = useState("");
  const canProbe = detail.numericScore === null && detail.officialGrade !== null && detail.classNumber !== null;
  const probe = async () => { try { setProbeState("running"); const score = await onProbe(detail.id); setProbeState("done"); setProbeMessage(`已验证教务数值：${score}`); } catch (error) { setProbeState("error"); setProbeMessage(String(error)); } };
  return <aside className="detail-panel" aria-labelledby="course-title"><button className="close-button" type="button" onClick={onClose} aria-label="关闭课程详情">×</button><p className="eyebrow">{detail.term}</p><h2 id="course-title">{detail.courseName}</h2><p className="course-code">{detail.courseCode} · {detail.category}</p><div className="grade-hero"><span>{scoreSource(detail)}</span><strong>{gradeLabel(detail)}</strong><small>{detail.gradePoint?.toFixed(1) ?? "—"} 绩点 · {detail.credit.toFixed(1)} 学分</small></div>{canProbe && <section className="numeric-probe"><h3>数值成绩</h3><p className="muted">仅探测这一门课程，最多发送 41 次顺序请求；不会自动运行或影响其他课程。</p>{probeState === "confirm" ? <div className="probe-actions"><button className="secondary-button" type="button" onClick={() => setProbeState("idle")}>取消</button><button className="primary-button" type="button" onClick={() => void probe()}>确认并开始</button></div> : <button className="secondary-button" type="button" disabled={probeState === "running"} onClick={() => setProbeState("confirm")}>{probeState === "running" ? "正在探测…" : "查询教务数值"}</button>}{probeMessage && <p className={probeState === "error" ? "probe-message error" : "probe-message"} role="status">{probeMessage}</p>}</section>}<section><h3>成绩构成</h3>{detail.components.length > 0 ? <div className="component-list">{detail.components.map((component) => <div key={component.name}><span>{component.name}</span><span>{component.score ?? "—"} · {component.weight ?? "—"}%</span></div>)}</div> : <p className="muted">教务未提供成绩构成。</p>}</section><section><h3>修读信息</h3><dl><div><dt>教学班</dt><dd>{detail.classNumber ?? "教务未提供"}</dd></div><div><dt>状态</dt><dd>{detail.passed ? "已通过" : "未通过"}</dd></div></dl></section></aside>;
}

function Archive({ runs, changes, onReview, onExport, onClear, onCreateArchive }: { runs: SyncRun[]; changes: ChangeRecord[]; onReview: () => void; onExport: (format: "json" | "csv") => void; onClear: () => void; onCreateArchive: () => void }) {
  return <section aria-labelledby="archive-title"><div className="page-heading" data-tauri-drag-region><div><p className="eyebrow">归档</p><h1 id="archive-title">本地历史</h1></div><div className="archive-actions"><button className="primary-button" type="button" onClick={onCreateArchive}>创建快照</button><button className="secondary-button" type="button" onClick={() => onExport("csv")}>导出 CSV</button><button className="secondary-button" type="button" onClick={() => onExport("json")}>导出 JSON</button></div></div>
    <div className="archive-grid"><section className="section-card"><div className="section-heading"><div><p className="eyebrow">待审阅</p><h2>检测到的变更</h2></div>{changes.length > 0 && <button className="text-button" type="button" onClick={onReview}>全部标记已审阅</button>}</div>
      {changes.length > 0 ? <div className="change-list">{changes.map((change) => <div key={change.id}><span><strong>{change.courseName}</strong><small>{change.courseCode} · {change.changeType === "course_added" ? "新增出分课程" : "已有成绩更新"}</small></span><time>{change.detectedAt}</time></div>)}</div> : <p className="muted padded">当前没有待审阅的新出分或成绩更新。</p>}
    </section><section className="section-card"><p className="eyebrow">隐私</p><h2>管理本机数据</h2><p className="archive-copy">导出文件会保存到应用数据目录。清除只影响此设备，不会修改教务系统。</p><button className="danger-button" type="button" onClick={onClear}>清除本地档案</button></section></div>
    <section className="table-card archive-table"><div className="section-heading"><div><p className="eyebrow">快照</p><h2>归档记录</h2></div></div>{runs.length > 0 ? runs.map((run) => <div className="run-row" key={run.id}><span><strong>本地快照 #{run.id}</strong><small>{run.sourceVersion}</small></span><span>{run.snapshotCount} 门课程</span><span>{run.changeCount} 项变更</span><time>{run.finishedAt}</time></div>) : <p className="empty-state">还没有本地快照。</p>}</section>
  </section>;
}

function Connection({ status, method, rankSummary, onMethod, onLogin, onVerify, onSync, onRank }: { status: JwxtStatus; method: GradeQueryMethod; rankSummary: RankSummary | null; onMethod: (method: GradeQueryMethod) => void; onLogin: () => void; onVerify: () => void; onSync: () => void; onRank: () => void }) {
  return <section aria-labelledby="connection-title"><div className="page-heading"><div><p className="eyebrow">连接教务</p><h1 id="connection-title">受控登录</h1></div></div><div className="connection-card"><p className="eyebrow">CAS · JWXT</p><h2>{status.connected ? "教务会话已认证" : "在应用内完成统一认证"}</h2><p>{status.message}</p><div className="archive-actions"><button className="primary-button" type="button" onClick={onLogin}>打开教务登录</button><button className="secondary-button" type="button" onClick={onVerify}>保存并验证会话</button></div><label className="query-method"><span>成绩查询方式</span><select value={method} onChange={(event) => onMethod(event.target.value as GradeQueryMethod)}><option value="officialList">官方成绩单</option><option value="achievementSearch">课程成绩检索</option></select></label><div className="archive-actions"><button className="secondary-button" type="button" onClick={onSync}>同步所选方式</button><button className="secondary-button" type="button" onClick={onRank}>查询排名统计</button></div>{rankSummary && <section className="rank-summary" aria-label="教务排名统计"><div><span>累计排名</span><strong>{rankSummary.totalRank ?? "—"}/{rankSummary.totalStudents ?? "—"}</strong></div><div><span>学期排名</span><strong>{rankSummary.termRank ?? "—"}/{rankSummary.totalStudents ?? "—"}</strong></div><div><span>累计绩点</span><strong>{rankSummary.cumulativeGpa ?? "—"}</strong></div><div><span>已获学分</span><strong>{rankSummary.earnedCredits ?? "—"}</strong></div></section>}<p className="muted">登录完成后切回 Grade Desk，点击“保存并验证会话”。应用会保存 Cookie 并自动关闭登录窗口。认证只验证 JWXT 会话，不依赖成绩是否可读。查询方式由你手动选择，仍受学校教务系统的业务规则约束。密码仅在教务登录页面中输入；Cookie 仅保存在本机受限文件中。</p></div></section>;
}

createRoot(document.getElementById("root")!).render(<App />);
