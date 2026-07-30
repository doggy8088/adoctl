# DEVELOPMENT_LOG.md

## 初始規劃與專案骨架

- 專案名稱選定為 `adoctl`，理由是名稱短、有力，符合 `kubectl`、`gh` 這類管理型 CLI 的命名習慣，並保留 ADO / Azure DevOps 辨識度。
- 語言選擇 Rust，目標是提供單一 binary、跨平台執行、低部署成本與良好型別安全。
- CLI 使用 `clap`，所有使用者可見訊息與 help 文字採繁體中文。
- HTTP client 使用 `reqwest` + `rustls`，降低 OpenSSL 在跨平台打包時的差異。
- 認證第一版支援 PAT、Azure CLI token、OAuth device code；PAT 流程只做引導、貼入、驗證與保存，不假設 CLI 能代替使用者產生 PAT。
- 憑證保存以 OS credential store 為目標，程式碼透過 `CredentialStore` trait 抽象，測試使用 memory store。
- User 參數統一使用 `--upn` 或 `--id`，並以 `UserIdentifier` 集中處理，避免各 command 重複實作。
- accessLevel 只開放 `stakeholder`、`basic`、`basic-test-plans`，並在 `access_level` 模組集中轉換 Azure DevOps API 使用的原始值。
- 專案成員管理預設群組為 `Contributors`，保留 `--group` 可覆寫。
- 打包流程透過 `cargo xtask` 集中，CI/CD 未建立 pipeline，但 `cargo xtask ci` 可作為未來 pipeline 入口。

## PAT 建立網址修正

- PAT 建立頁面必須包含 organization path，例如 `https://dev.azure.com/willh/_usersSettings/tokens`。
- `login` 流程改為使用 `--org` 解析後的 organization 名稱動態產生 PAT 建立網址。
- 若使用者傳入 `https://dev.azure.com/willh`，會先解析成 `willh`，再產生正確 PAT 網址。

## 登入憑證持久化與 help 行為修正

- `keyring` crate 預設不啟用平台原生儲存後端；在 macOS 若未開啟 `apple-native` 會使用 mock store，只能在單一行程內保存資料。
- 專案改為啟用 `apple-native`、`windows-native`、`linux-native-sync-persistent` 與 `crypto-rust`，讓 `adoctl login` 保存後可被後續 CLI 執行讀取。
- PAT scope 提示用語改為 Azure DevOps 文件名稱 `Member Entitlement Management`。
- 根命令與命令群組加入不帶參數時顯示 help 的行為，避免只顯示格式錯誤。

## Azure DevOps Extension PAT 環境變數支援

- PAT 來源新增 `AZURE_DEVOPS_EXT_PAT`，方便沿用 Azure DevOps Extension / Azure CLI extension 既有環境設定。
- 認證載入順序為 `--pat` / `ADOCTL_PAT` 優先，其次 `AZURE_DEVOPS_EXT_PAT`，最後才讀取 OS credential store。
- `login --method pat` 未指定 `--pat` 時也會先讀取環境變數，若不存在才互動提示貼上 PAT。

## user list 全量讀取修正

- Member Entitlement Management `userentitlements` list API 可能只回傳單頁資料，因此 `adoctl user list` 改為使用 `top=100` / `skip` 分頁讀取。
- 分頁取完後才套用 `--access-level` 與 `--search`，確保過濾條件不會只作用在第一頁。
- 新增 mock API 測試，確認第二頁的使用者也會被列入結果。

## 子命令名稱單數化

- 對外 CLI 子命令由 `users` / `projects` 改為 `user` / `project`，讓命令階層與使用者回饋一致。
- 同步更新 `clap` command name、help usage、CLI integration test 與 README 範例。

## `--debug` 旗標與 `user list` 回應格式修正

- 新增全域 `--debug` 旗標，除錯資訊統一輸出到 `stderr`，避免破壞 `stdout` 的 table / JSON 穩定格式。
- 除錯訊息目前涵蓋：認證來源摘要、HTTP request URL、HTTP status、JSON 回應摘要、`user list` 分頁與過濾統計；刻意不輸出 PAT、access token、refresh token 或 Authorization header。
- `userentitlements` list response 原本只反序列化 `members` 欄位，實際 Azure DevOps API 會回傳 `items`；因此 `adoctl user list` 會誤判成空陣列。
- `UserEntitlementList` 改為支援 `items` / `members` / `value` alias，讓 CLI 能相容不同 collection key，同時保留既有測試相容性。
- 受影響模組：`src/cli.rs`、`src/debug.rs`、`src/auth.rs`、`src/ado/client.rs`、`src/ado/users.rs`、`tests/cli_help.rs`、`tests/users_client.rs`、`README.md`。
- 測試策略：
  - CLI help test 驗證 `--debug` 有出現在 help。
  - CLI integration test 驗證 `--debug` 會把詳細診斷資訊寫到 `stderr`，且 `stdout` JSON 仍可供自動化使用。
  - API client / user list 測試改用實際 `items` collection shape，並在單元測試補 `members` alias 反序列化驗證。
- 驗證結果：本機 `cargo xtask test` 已通過；另外以實際環境執行 `cargo run -- --debug user list --output json` 可看到 HTTP 回應摘要，並確認 `items.len` 與最終列出的使用者數量一致。

## `project list` 命令與專案過濾

- 新增 `adoctl project list`，讓 CLI 不只管理專案成員資格，也能直接列出 organization 內的專案。
- `project list` 支援兩種過濾：
  - `--state <狀態>`：使用 Azure DevOps Projects API 的 `stateFilter` 做伺服器端過濾。
  - `--search <關鍵字>`：在 CLI 端依專案名稱、描述或 Id 做關鍵字搜尋。
- 新增 `ProjectStateFilter` 型別集中管理 CLI 值與 Azure DevOps API 值的對應，避免 `wellFormed`、`createPending` 這類 API 值散落在程式碼中。
- Projects list API 透過 response header `x-ms-continuationtoken` 分頁，因此 `AdoClient` 新增 `get_json_with_headers`，保留 response header 給上層命令處理。
- `Project` response 型別補齊 `description`、`state`、`visibility`、`url`、`lastUpdateTime` 等欄位，JSON 輸出可供自動化串接，table 輸出則聚焦顯示名稱、Id、狀態、可見性與最後更新時間。
- 受影響模組：`src/project_state.rs`、`src/cli.rs`、`src/commands/mod.rs`、`src/ado/client.rs`、`src/ado/projects.rs`、`src/output.rs`、`tests/cli_help.rs`、`tests/projects_client.rs`、`README.md`。
- 測試策略：
  - CLI help test 驗證 `project list --help` 有列出 `--state` / `--search`。
  - API client / command integration test 驗證 `project list` 可沿用 `AZURE_DEVOPS_EXT_PAT`、可依 state + search 過濾，且能依 continuation token 讀完多頁結果。
  - 單元測試驗證 continuation token header 解析與 project state enum mapping。
- 驗證結果：完成後應以 `cargo xtask test` 驗證；若本機已有登入資訊，可再用 `cargo run -- --debug project list --output json` 觀察實際 API request / response 摘要。

## Rust CLI 開發 agent skill

- 於專案層級新增 `.agents/skills/rust-cli-development/`，提供未來 agent 在 Rust CLI 任務中可重複套用的工作流程與檢查清單。
- 技術決策：
  - 採用 Agent Skills 標準規定的目錄型 skill，使用 `SKILL.md` 作為入口，而不是直接在 `.agents/skills/` 根目錄放單一 `.md`。
  - `description` 以英文撰寫，提升 system prompt 中的技能匹配率；skill 內容本體以繁體中文撰寫，方便本地使用與維護。
  - 另外拆出 `references/checklists.md`，把詳細 checklist 與實作藍圖從主 skill 拆開，維持 `SKILL.md` 可讀性。
- 內容範圍包含：命令 UX 設計、模組切分、serde 型別、stdout/stderr 契約、測試矩陣、API 命令實作注意事項、debug/logging guardrails、發布前 checklist。
- 特別強調：
  - 先讀 repo 的 `AGENTS.md` / `README.md` / `Cargo.toml` / `tests/`
  - 優先沿用既有 `xtask`、腳本與錯誤處理方式
  - 禁止將 debug 訊息混入 `stdout` JSON
  - 不可在 log、測試或文件中洩漏敏感資訊
- 受影響檔案：
  - `.agents/skills/rust-cli-development/SKILL.md`
  - `.agents/skills/rust-cli-development/references/checklists.md`
- 驗證方式：
  - 確認 skill 名稱 `rust-cli-development` 與資料夾名稱一致。
  - 確認使用目錄型 skill 結構，符合 `.agents/skills/` 的 discovery 規則。
  - 確認 `SKILL.md` 中的相對連結可正確指向 `references/checklists.md`。

## 代理程式集區、代理程式與工作查詢

- 新增 `adoctl pool` 命令群組：
  - `pool list [--pool-type automation|deployment]`
  - `pool agents --pool <集區名稱或 Id>`
  - `pool jobs --pool <集區名稱或 Id>`
- 技術決策與理由：
  - 集區清單使用 Distributed Task Pools API 7.1；`--pool-type` 直接映射為 API 的 `poolType` query，避免先下載全部資料再於本機過濾。
  - `PoolTypeFilter` 集中處理 `automation` / `deployment` 的 CLI 驗證與 API mapping。
  - `--pool` 同時接受名稱與數字 Id。名稱會先透過 `poolName` query 查詢，再以不分大小寫的完整名稱比對；零筆與同名多筆都回報明確錯誤。
  - 代理程式清單使用 Agents List API 7.1，並設定 `includeAssignedRequest=true`，讓輸出除 `online` / `offline` 外，也能顯示目前指派工作。
  - 工作清單使用 Distributed Task `jobrequests` 的分頁介面與 `7.1-preview.1`。此端點出現在 Microsoft Azure DevOps 官方 SDK，但未列入公開 REST 7.1 參考頁；因此將 response 欄位限制在已型別化的穩定工作要求資料，並接受 wrapped collection 與直接陣列兩種回應形狀。
  - `pool jobs` 使用 `$top=100` 與 `x-ms-continuationtoken` 讀完所有頁面，而不是使用只能限制近期完成數量的 `completedRequestCount` 查詢。
  - continuation token 解析移至 `ado::client` 共用 helper，供 Projects 與工作清單共用；工作分頁另偵測重複 token，避免服務異常時形成無限迴圈。
- 輸出契約：
  - `--output json` 直接輸出型別化陣列，保留 API 的欄位名稱與原始 enum 值，供自動化串接。
  - table 模式將 pool type、代理程式狀態、工作狀態與結果翻譯為繁體中文。
  - 正常結果只寫入 `stdout`；HTTP 與分頁診斷只在 `--debug` 時寫入 `stderr`。
- 受影響模組與 API：
  - `src/pool_type.rs`
  - `src/ado/pools.rs`
  - `src/ado/client.rs`
  - `src/cli.rs`
  - `src/commands/mod.rs`
  - `src/output.rs`
  - `src/error.rs`
  - `tests/pools_client.rs`
  - `tests/cli_help.rs`
  - `README.md`
- 測試策略：
  - 驗證 `poolType`、`poolName`、`includeAssignedRequest`、`$top`、`continuationToken` 與 API version 的 HTTP request shape。
  - 驗證集區清單、代理程式狀態、目前工作、wrapped/direct 工作清單回應與多頁工作讀取。
  - 驗證集區不存在、HTTP 5xx 與 CLI 缺少 `--pool` 的錯誤路徑。
  - 驗證 table 模式會以「上線」顯示代理程式狀態。
- 驗證結果：`cargo xtask test` 已通過，包含 `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings` 與 `cargo test --workspace`。

## Makefile 常用開發工作

- 新增根目錄 `Makefile`，提供 `help`、`build`、`release`、`check`、`fmt`、`fmt-check`、`clippy`、`test`、`test-unit`、`ci`、`run`、`doc`、`install`、`package`、`package-all` 與 `clean`。
- 技術決策與理由：
  - `make test` 與 `make ci` 直接委派給既有 `cargo xtask`，避免 Makefile 與 xtask 各自維護一套品質檢查流程。
  - `make package TARGET=<target-triple>` 與 `make package-all` 直接委派給 xtask 打包流程，保留原有 target 清單與產物命名規則。
  - `CARGO ?= cargo` 允許開發環境覆寫 Cargo 執行檔，同時維持一般環境零設定。
  - `make run ARGS="..."` 只負責轉送 CLI 參數，不保存 organization、認證或其他環境資訊。
  - `make install` 使用 `cargo install --root`，預設安裝根目錄為 `~/.local`，因此 binary 會放在 `~/.local/bin/adoctl`；`--force` 確保同版本的本機原始碼變更也會重新安裝。
  - 安裝根目錄可透過 `LOCAL_INSTALL_ROOT` 覆寫，但不修改或覆寫系統的 `HOME` 環境變數。
- 受影響檔案：`Makefile`、`README.md`、`DEVELOPMENT_LOG.md`、`DEBUGGING_NOTES.md`。
- 測試策略：
  - 執行 `make help` 驗證 target 清單與繁體中文說明。
  - 執行 `make check` 驗證一般建置檢查。
  - 執行 `make test` 驗證 Makefile 能完整委派既有格式、Clippy 與測試流程。
  - 以 dry-run 驗證 `run` 與指定 target 打包的參數轉送。
  - 以暫存安裝根目錄實際執行 `make install`，確認 binary 安裝於 `<root>/bin/adoctl`，避免驗證時覆寫使用者現有的 `~/.local/bin/adoctl`。
- 驗證結果：
  - `make help` 正確列出所有 target 與繁體中文說明。
  - `make check` 通過。
  - `make package` 未提供 `TARGET` 時依設計以 exit code 2 失敗，並顯示明確用法。
  - `make -n run ARGS='--help'` 與 `make -n package TARGET=x86_64-apple-darwin` 的參數轉送正確。
  - `make install LOCAL_INSTALL_ROOT=<暫存目錄>` 實際安裝到 `<暫存目錄>/bin/adoctl`，執行 `adoctl --version` 回傳 `adoctl 0.1.0`。
  - `make test` 通過，包含格式檢查、Clippy 零警告與全部 workspace 測試。
