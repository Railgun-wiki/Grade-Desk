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
  const [activeView, setActiveView] = useState<"overview" | "transcript">("overview");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [detail, setDetail] = useState<CourseDetail | null>(null);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void invoke<AppStatus>("application_status").then(setStatus).catch(() => {
      setStatus({ name: "Grade Desk", version: "Web preview", storageMode: "local-only" });
    });
  }, []);

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
      <header className="context-nav"><span>成绩</span><span className="term-chip">{dashboard.currentTerm}</span><button className="primary-button" type="button" disabled>同步成绩</button></header>
      <aside className="sidebar" aria-label="主导航">
        <button className={activeView === "overview" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("overview")} type="button">概览</button>
        <button className={activeView === "transcript" ? "nav-item active" : "nav-item"} onClick={() => setActiveView("transcript")} type="button">成绩单</button>
        <button className="nav-item" type="button" disabled>分析 <span>即将推出</span></button>
        <button className="nav-item" type="button" disabled>归档 <span>即将推出</span></button>
      </aside>
      <main className="content" id="main-content">
        {activeView === "overview" ? <Overview dashboard={dashboard} attempts={attempts} onTranscript={() => setActiveView("transcript")} /> : (
          <Transcript attempts={filteredAttempts} query={query} onQuery={setQuery} onSelect={setSelectedId} />
        )}
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

createRoot(document.getElementById("root")!).render(<App />);
