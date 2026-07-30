---
name: rust-cli-development
description: Develop and maintain Rust CLI tools. Use when scaffolding a new CLI, adding commands, flags, or subcommands, designing clap-based UX, implementing config/auth/output handling, building API clients, writing parser/help/integration tests, debugging command behavior, or packaging a Rust command-line application.
compatibility: Requires a Rust/Cargo codebase plus the ability to read files, edit code, and run Cargo commands.
---

# Rust CLI Development

此 skill 用來協助開發與維護 Rust CLI 工具。載入後，請優先遵循目前 repo 的既有慣例，而不是強行導入新的框架或目錄結構。

## 何時使用

當任務包含下列情境時，應使用這個 skill：

- 建立新的 Rust CLI 專案或子命令
- 新增 `--flag`、參數、subcommand、alias、互斥規則
- 設計 `clap` / 命令列 UX / `--help` 文案
- 實作設定檔、環境變數、credential/profile 載入
- 實作 API client、分頁、認證、輸出格式
- 新增 `table` / `json` / `yaml` 等輸出模式
- 撰寫 parser/help/integration/mock-server 測試
- 除錯 CLI 執行結果、空輸出、HTTP 回應格式落差、分頁問題
- 打包、發布、安裝與跨平台執行驗證

## 先做這些事

1. 讀取並遵守 repo 的 `AGENTS.md`、`README.md`、`Cargo.toml`、`src/main.rs`、`src/lib.rs`、`tests/`。
2. 找出 CLI parser 的既有實作方式（例如 `clap` derive、builder API、或其他 crate）。
3. 找出既有輸出模式、錯誤型別、設定檔、日誌與測試入口。
4. 若 repo 有 `xtask`、`Makefile`、`justfile`、`scripts/` 或 CI 命令，優先沿用。
5. 若需求涉及使用者可見訊息、範例、`--help` 或錯誤文案，務必遵守該 repo 的語言與文案規範。

## 預設技術選擇

若 repo 尚未定案，優先使用以下組合；若 repo 已有慣例，則以既有慣例為主：

- CLI：`clap` derive API
- 錯誤處理：`thiserror` 做領域錯誤；只在 binary 邊界使用 `anyhow`（若專案本來就採用）
- 非同步：`tokio`
- HTTP：`reqwest` + `rustls`
- 序列化：`serde` + `serde_json`
- 測試：`assert_cmd`、`predicates`、`tempfile`
- HTTP mock：`wiremock` 或 repo 既有工具
- 日誌：`tracing` 或明確的 `--debug`；不要把除錯輸出混入機器可讀的 `stdout`

## 實作流程

### 1. 先設計命令 UX，再開始寫程式

先定義：

- 命令 / 子命令名稱
- 參數名稱與型別
- 預設值
- 互斥 / 相依規則
- 對應環境變數
- 成功輸出與錯誤訊息
- 至少 2～3 個實際使用範例

若是自動化會用到的命令，預設要考慮穩定的 `--output json`。

### 2. 保持模組邊界清楚

建議的責任切分：

- `cli`：參數與 help 定義
- `commands`：命令處理流程
- `client` / `ado` / `api`：API 呼叫與資料轉換
- `output`：table/json renderers
- `error`：型別化錯誤
- `config`：環境變數 / 設定檔 / profile

不要把參數解析、商業邏輯、HTTP 呼叫與輸出格式全部混在 `main.rs`。

### 3. 先建立資料模型，再寫 API 邏輯

- 先用 `serde` 型別描述 request / response
- 避免直接到處操作未型別化的 `serde_json::Value`
- API 的特殊格式、alias、分頁 token、欄位落差，集中在 client 或 model 層處理
- 若 API 有多種 collection key（例如 `items` / `value` / `members`），集中在型別層解決

### 4. 維持乾淨的 stdout / stderr 契約

- 正常結果輸出到 `stdout`
- 錯誤與除錯資訊輸出到 `stderr`
- `--output json` 時，`stdout` 只能是 JSON
- 不可輸出 token、PAT、cookie、private key、Authorization header 或其他敏感資訊

### 5. 測試使用者契約，而不只測內部函式

最低限度要覆蓋：

- 參數解析
- `--help` 文案
- 成功路徑
- 缺參數 / 無效參數
- 空結果
- `--output json` 的 shape
- HTTP request shape（若有 API）
- 404 / 409 / 401 / 5xx 等錯誤對映
- 分頁 / continuation token / alternate response shape

### 6. 同步更新文件與範例

若有以下內容，應一併更新：

- `README.md`
- 使用範例
- help snapshot / CLI 測試
- 開發紀錄 / changelog / debugging notes（若 repo 有要求）

## API 型 CLI 的額外規則

- 使用 mock server 測成功、失敗、找不到、request shape、分頁與回應格式差異
- 若服務把分頁資訊放在 response header，而不是 body，應擴充共用 client helper，而不是在命令層各自硬寫
- 若需要 debug log，至少應記錄：request URL、status、回應摘要、分頁資訊；但絕不可印出敏感資訊
- 若命令表面上成功但結果為空，優先檢查：
  - collection key 是否對得上
  - filter 是否套在錯的階段
  - 是否只讀到第一頁
  - stdout 是否被 debug log 汙染

## 實作守則

- 優先延伸既有抽象，不要平行再造一套
- 優先提供明確錯誤，不要靜默 fallback 掩蓋問題
- 新增 debug 功能時，必須可關閉，且不影響既有自動化輸出
- JSON 輸出要穩定，避免破壞腳本與 CI
- 若 repo 有在地化規範，所有使用者可見訊息都要遵守

## 完成前檢查清單

- [ ] 命令 / 參數已接入 CLI
- [ ] help 文案完整且一致
- [ ] handler 已接線
- [ ] 輸出 renderer 已完成
- [ ] 錯誤訊息明確
- [ ] 測試已補齊
- [ ] README / 範例已更新
- [ ] 除錯輸出不含敏感資訊
- [ ] `cargo fmt`、`cargo clippy`、`cargo test` 已通過

## 常用命令

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -- --help
```

如果 repo 有專屬工作流程（例如 `cargo xtask test`），請優先使用專案既有命令。

## 進一步參考

需要更細的工作清單與常見實作藍圖時，請再讀：

- [詳細檢查清單與實作藍圖](references/checklists.md)
