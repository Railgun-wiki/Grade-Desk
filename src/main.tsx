import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

type AppStatus = {
  name: string;
  version: string;
  storageMode: string;
};

function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);

  useEffect(() => {
    void invoke<AppStatus>("application_status").then(setStatus).catch(() => {
      setStatus({ name: "Grade Desk", version: "Web preview", storageMode: "local-only" });
    });
  }, []);

  return (
    <main className="shell">
      <section className="hero" aria-labelledby="app-title">
        <p className="eyebrow">个人成绩档案</p>
        <h1 id="app-title">Grade Desk</h1>
        <p className="lead">清晰查看每一次努力。</p>
        <p className="description">成绩、分析与归档将只保存在这台设备。</p>
        <div className="status" aria-live="polite">
          <span className="status-dot" />
          <span>{status ? `${status.name} ${status.version} · ${status.storageMode}` : "正在启动本地工作区…"}</span>
        </div>
      </section>
      <section className="next" aria-labelledby="next-title">
        <p className="eyebrow">下一步</p>
        <h2 id="next-title">建立本地成绩档案</h2>
        <p>SQLite 数据层将在下一模块中创建，随后才会添加成绩概览和教务同步。</p>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
