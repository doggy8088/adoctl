# 變更紀錄

本文件記錄 `adoctl` 的重要變更。格式參考 [Keep a Changelog](https://keepachangelog.com/zh-TW/1.1.0/)，版本編號遵循 [語意化版本](https://semver.org/lang/zh-TW/)。

維護新變更時，先加入「尚未發布」段落；準備發布時，再移至對應版本並補上日期。建議使用「新增」、「變更」、「修正」、「移除」、「安全性」分類，沒有內容的分類不必保留。

* * *

## [尚未發布]

### 新增

- 新增不依賴 install script 的薄封裝 `adoctl` npm wrapper，第一次執行時依作業系統、CPU 架構與 Linux libc 下載並選擇六平台 Rust binary。
- 新增 npm 初始本機封裝、安裝、首次發布與 Trusted Publishing 完整設定文件。
- 新增 npm wrapper、平台 mapping、Release 資產與 checksum 測試。

### 變更

- GitHub Actions CI 新增 Node.js、npm wrapper 與 npm tarball 清單驗證。
- 標籤式發布流程新增 Cargo／npm 版本一致性檢查，並在 GitHub Release 完成後透過 OIDC Trusted Publishing 發布具 provenance 的 npm 套件。

* * *

## [0.1.0] - 2026-07-31

### 新增

- 建立以 Rust、`clap`、`tokio`、`reqwest` 與 `rustls` 實作的 Azure DevOps 管理 CLI。
- 支援 PAT、Azure CLI token 與 OAuth device code 認證。
- 支援 organization 使用者查詢、accessLevel 過濾及授權變更。
- 支援 Stakeholder、Basic、Basic + Test Plans、Visual Studio Subscriber、Visual Studio Enterprise 與 GitHub Enterprise 查詢。
- 支援 Azure DevOps 專案清單及專案群組成員新增、移除。
- 支援代理程式集區型別過濾、代理程式狀態及工作要求查詢。
- 提供 table 與穩定 JSON 輸出，以及不污染正常輸出的 `--debug` 診斷模式。
- 提供 `Makefile`、`cargo xtask`、本機安裝與六個預設 target 的封裝流程。
- 新增 GitHub Actions 持續整合，在 `main` push 與 pull request 執行格式、Clippy 與完整測試。
- 新增標籤式發布流程，在推送與 Cargo 版本一致的 `v<版本>` 標籤後建立跨平台封裝及 GitHub Release。

### 變更

- GitHub repository 由 `ado-manager` 重新命名為 `adoctl`，使 repository 名稱與 CLI binary 一致。

### 修正

- 修正 Azure DevOps `userentitlements` 清單的 collection key 與分頁處理，避免成功回應被解析成空清單。
- 修正 PAT 建立網址缺少 organization path 的問題。
- 修正 macOS credential store 未啟用原生後端造成登入資訊無法跨行程保存的問題。
- 修正 GitHub Actions Linux runner 缺少 D-Bus 開發套件造成 Clippy 建置失敗的問題。
- 修正 accessLevel API mapping、訂閱授權欄位及更新回應 wrapper。

[尚未發布]: https://github.com/doggy8088/adoctl/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/doggy8088/adoctl/releases/tag/v0.1.0
