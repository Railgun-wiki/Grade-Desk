# Grade Desk (成绩桌面客户端)

Grade Desk 是一个专为高校学生设计的**本地优先 (Local-first)、隐私安全**的成绩管理、绩点分析与快照归档桌面客户端。

> [!IMPORTANT]
> **免责声明 (Disclaimer)**：
> 本项目是一个独立开发的开源成绩管理工具，仅供个人学习、学业分析与学术研究使用。**本项目与任何高校或官方教务机构无任何关联，非官方发布软件。** 软件中涉及的教务对接机制仅在本地运行，请在遵守所在学校网络及系统使用规范的前提下使用。

项目基于现代化的 **Tauri 2 + Vite + React + Rust + SQLite** 架构构建，旨在为学生提供一份完全独立于教务系统、自主掌控的本地学业档案。

---

## 🌟 核心特性

- **本地优先与隐私安全**：成绩档案均存储在本地 SQLite 数据库中。所有教务认证会话 (CAS Cookies) 仅限内存或受限文件内使用，不上传至任何第三方服务器。
- **精致统一的视觉风格**：统一沿用 Apple 风格的设计语言（精细的卡片式布局、平滑圆角、柔和微交互、Apple 经典蓝色），在各种操作系统上呈现精美的界面。
- **多平台原生窗口适配**：
  - **macOS**：支持毛玻璃 sidebar (NSVisualEffect) 并完美融入 native traffic lights。
  - **Windows**：支持原生 Windows 11 Mica 效果并适配原生窗口边框。
  - **Linux**：自动禁用不稳定的透明度效果，回退为不透明背景，确保极致的视觉稳定性。
- **自动变更追踪**：记录每一次的成绩变更（如未出分到已出分、教务分数改动），自动生成本地时间轴快照。
- **数据自主控制**：支持一键导出 CSV/JSON 数据，或物理清空本机上的所有数据。

---

## 🛠️ 开发环境要求

- **Node.js** (LTS 18/20+)
- **pnpm** (唯一指定前端依赖管理器)
- **Rust 工具链** (Rustc 1.75+)
- 当前平台特有的编译依赖（参见 [开发者文档](docs/developer-documentation.md#3-本地开发环境搭建)）

---

## 🚀 快速开始

### 1. 安装前端依赖
```bash
pnpm install
```

### 2. 运行开发模式
```bash
pnpm tauri dev
```

### 3. 本地编译打包 (生成当前系统安装包)
```bash
pnpm tauri build
```

---

## 📖 项目文档与开发规范

在您开始贡献或开发代码前，请务必参阅以下文档：

- ⚙️ **[开发者核心文档 (Developer Guide)](docs/developer-documentation.md)**：包含整体系统架构设计、详细目录介绍、本地环境配制、性能限制及安全原则。
- 📦 **[各功能模块说明文档 (Module Specs)](docs/modules/)**：
  - [应用外壳与基础配置 (App Shell)](docs/modules/app-shell.md)
  - [成绩持久化仓库 (Grade Repository)](docs/modules/grade-repository.md)
  - [成绩概览与主看板 (Grade Dashboard)](docs/modules/grade-dashboard.md)
  - [历史版本与归档流程 (Archive Workflow)](docs/modules/archive-workflow.md)
  - [教务认证与数据抓取 (JWXT Session)](docs/modules/jwxt-session.md)
  - [系统日志与诊断 (Logging)](docs/modules/logging.md)
  - [跨平台兼容与自动化流水线 (CI/Release)](docs/modules/ci-release-compatibility.md)

---

## 🤝 开发约定

- **Commit 提交格式**：遵循 Conventional Commits 规范，例如 `feat(ui): add overview page` 或 `fix(db): resolve migration panic`。
- **包依赖管理**：严格使用 `pnpm`。不得提交 `package-lock.json` 或 `yarn.lock`。
- **隐私防线**：任何代码提交绝对不可泄露学校账号、明文凭证、私钥及生成的数据库文件。

---

## 📝 开源协议

本项目属于面向大学生的本地成绩档案管理辅助工具，具体协议请参见 LICENSE。数据及知识产权归项目开发者和贡献者所有。
