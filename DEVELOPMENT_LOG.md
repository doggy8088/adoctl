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

## GitHub Actions 持續整合

- 新增 `.github/workflows/ci.yml`，在 `main` push、pull request 與手動觸發時執行。
- 技術決策與理由：
  - CI 直接執行 `cargo xtask ci`，與本機 `make ci`、`make test` 共用相同的格式、Clippy 與測試流程，避免本機與遠端檢查分歧。
  - 使用 GitHub-hosted `ubuntu-latest` runner 與 Rust stable toolchain，明確安裝 `rustfmt` 與 `clippy` 元件。
  - `actions/checkout` 採用官方 `v6.0.2` 對應的完整 commit SHA，符合 GitHub 對不可變 action 版本的安全建議。
  - checkout 設定 `persist-credentials: false`，CI 不需要在工作目錄保留 GitHub 認證。
  - Ubuntu runner 明確安裝 `libdbus-1-dev` 與 `pkg-config`，滿足 `keyring` Linux 原生持久化後端經由 `libdbus-sys` 使用的系統建置相依套件。
  - workflow 權限只保留 `contents: read`，不提供寫入 repository、pull request、package 或其他資源的權限。
  - concurrency 依 workflow 與 ref 分組，新提交會取消同分支尚未完成的舊 run，減少不必要的 runner 使用量。
  - 設定 20 分鐘 timeout，避免 runner 因非預期卡住而無限占用。
- 受影響檔案：
  - `.github/workflows/ci.yml`
  - `README.md`
  - `DEVELOPMENT_LOG.md`
  - `DEBUGGING_NOTES.md`
- 測試策略：
  - 本機解析 YAML 並檢查 workflow 必要欄位。
  - 執行 `cargo xtask ci`，確認與 workflow 使用的命令一致。
  - 推送至 GitHub 後監看實際 Actions run，確認 checkout、toolchain 與完整 CI job 成功。
- 初次 GitHub 驗證：
  - run `30558515298` 已通過 checkout、Rust toolchain 安裝與版本檢查，但在 `cargo xtask ci` 的 Clippy 建置階段失敗。
  - 錯誤來自 `libdbus-sys v0.2.7`，其 build script 透過 `pkg-config` 找不到 Ubuntu 的 `dbus-1` 開發檔案。
  - 本機 macOS 使用不同的 `keyring` 平台後端，因此本機完整測試成功仍無法涵蓋此 Linux runner 相依性。
  - workflow 已補上 `libdbus-1-dev` 與 `pkg-config` 安裝步驟。
- 最終驗證結果：
  - 本機 YAML 語法解析與 `git diff --check` 通過。
  - 本機 `cargo xtask ci` 通過，包含格式檢查、Clippy 零警告與 50 項測試。
  - GitHub Actions run [`30558727576`](https://github.com/doggy8088/adoctl/actions/runs/30558727576) 已在 Ubuntu runner 通過全部步驟，執行時間 2 分 9 秒。

* * *

## README 命令參考與 accessLevel 完整支援

- README 已為所有現有命令補上用途、語法、重要選項及可複製範例：
  - `login`
  - `user list`
  - `user get`
  - `user set-access`
  - `project list`
  - `project add-user`
  - `project remove-user`
  - `pool list`
  - `pool agents`
  - `pool jobs`
- 官方資料查核：
  - 研究結果保存於 `docs/research/azure-devops-access-levels.md`，只引用 Microsoft Learn、Microsoft 官方 REST 規格與 Microsoft／Azure 官方 repository。
  - 現行公開 access level 包含 Stakeholder、Basic、Basic + Test Plans、Visual Studio Subscriber、Visual Studio Enterprise 與 GitHub Enterprise。
  - REST `AccountLicenseType` 原始 enum 另有 `none`、`earlyAdopter` 與 `professional`；`none` 是複合 mapping 使用的 sentinel，`earlyAdopter` 明訂為 Microsoft 內部值，`professional` 在目前官方程式化 mapping 查無公開語意，因此不列為一般 CLI 選項。
- CLI 型別與行為：
  - `AccessLevel` 支援六種公開層級，用於 `user list --access-level` 過濾。
  - `AssignableAccessLevel` 只允許五種具有公開、可驗證直接指派 mapping 的層級，供 `user set-access` 使用。
  - `express` 作為 `basic` 的官方 API 別名，`advanced` 作為 `basic-test-plans` 的官方 API 別名；help 與 README 同時列出 canonical 名稱及別名。
  - `github-enterprise` 只支援查詢過濾，不允許直接設定。Azure DevOps 會在登入後自動偵測 GitHub Enterprise 權益；REST 7.1 尚無 `gitHubLicenseType`，7.2-preview.5 雖有 schema 欄位，但官方沒有 GitHub Enterprise PATCH request 範例。
- API mapping：
  - Stakeholder：`accountLicenseType=stakeholder`、`licensingSource=account`。
  - Basic：`accountLicenseType=express`、`licensingSource=account`。
  - Basic + Test Plans：`accountLicenseType=advanced`、`licensingSource=account`。
  - Visual Studio Subscriber：`accountLicenseType=none`、`licensingSource=msdn`、`msdnLicenseType=eligible`。
  - Visual Studio Enterprise：`accountLicenseType=none`、`licensingSource=msdn`、`msdnLicenseType=enterprise`。
- API client 調整：
  - `user list` 保留既有 `7.1-preview.4` 分頁契約。
  - 單一使用者 GET 與 PATCH 改用官方穩定的 REST `7.1`。
  - PATCH response 依官方 `UserEntitlementsPatchResponse.userEntitlement` wrapper 反序列化，不再假設 response 是裸 `UserEntitlement`。
  - `AccessLevelInfo` 新增 `licensingSource`、`msdnLicenseType` 與 `githubLicenseType` 型別化欄位，使訂閱型 access level 可正確過濾。
- 受影響檔案：
  - `src/access_level.rs`
  - `src/ado/users.rs`
  - `src/cli.rs`
  - `tests/cli_help.rs`
  - `tests/users_client.rs`
  - `README.md`
  - `docs/research/azure-devops-access-levels.md`
  - `DEVELOPMENT_LOG.md`
  - `DEBUGGING_NOTES.md`
- 測試策略：
  - 單元測試驗證六種公開 access level 的解析及 API response matching。
  - 單元測試確認 `none`、`earlyAdopter`、`professional` 不會成為一般選項，且 GitHub Enterprise 不可直接設定。
  - CLI help 測試分別驗證 `user list` 的六種過濾值及 `user set-access` 的五種可設定值。
  - mock server 測試逐一驗證五種可設定層級的 JSON Patch request shape、REST 7.1 query 與 wrapped response。
- 驗證結果：
  - `git diff --check` 通過。
  - `cargo xtask test` 通過，包含格式檢查、Clippy 零警告與全部 55 項測試。
  - 五種可直接設定的 access level 均已驗證 REST 7.1 JSON Patch request shape 與 wrapped response。

* * *

## CHANGELOG 與標籤式 GitHub Release

- 新增 `CHANGELOG.md`：
  - 採用 Keep a Changelog 的段落結構與語意化版本編號。
  - 保留「尚未發布」段落，供後續功能、變更、修正、移除及安全性項目持續累積。
  - 建立 `0.1.0` 初始版本紀錄，涵蓋 CLI、認證、使用者、專案、代理程式集區、測試、封裝與 CI。
- 新增 `.github/workflows/release.yml`：
  - 只在推送 `v*` 標籤時觸發，不擴大既有 `ci.yml` 的觸發條件或寫入權限。
  - 發布前先確認標籤必須等於 `v` 加上 `Cargo.toml` 的 `package.version`。
  - 要求 CHANGELOG 存在 `## [版本] - 日期` 標題，避免發布沒有人工整理版本紀錄的標籤。
  - 在建立封裝前執行 `cargo xtask ci`，維持與本機及既有 CI 相同的格式、Clippy 與測試入口。
  - 使用 GitHub-hosted 原生架構 runner，為六個既有預設 target 建立封裝：
    - `x86_64-pc-windows-msvc`
    - `x86_64-unknown-linux-gnu`
    - `x86_64-unknown-linux-musl`
    - `aarch64-unknown-linux-gnu`
    - `x86_64-apple-darwin`
    - `aarch64-apple-darwin`
  - Windows 產物使用 ZIP；Linux 與 macOS 產物使用 tar.gz。每個封裝包含 `adoctl`、`README.md` 與 `CHANGELOG.md`。
  - 每個 runner 都會執行封裝內的 `adoctl --version`，確認 binary 版本與標籤一致。
  - 最終 job 合併六個 artifact、產生 `SHA256SUMS`，再從 CHANGELOG 擷取對應版本內容作為 release notes。
  - 含預發布識別碼的版本會建立 GitHub prerelease；穩定版本則依 GitHub 預設規則標示 Latest。
- 權限與供應鏈決策：
  - workflow 預設只有 `contents: read`；只有最終 release job 取得 `contents: write` 與讀取同次 workflow artifact 所需的 `actions: read`。
  - 使用 runner 內建的 GitHub CLI 執行 `gh release create --verify-tag`，禁止 workflow 在標籤不存在時代為建立標籤。
  - `actions/checkout` 固定至 `v6.0.2` commit SHA。
  - `actions/upload-artifact` 固定至 `v7.0.1` commit SHA。
  - `actions/download-artifact` 固定至 `v8.0.1` commit SHA。
- Linux musl 技術處理：
  - `keyring` 的 Linux 原生後端會間接依賴 `libdbus-sys`。
  - GNU Linux 使用 runner 的 `libdbus-1-dev`；musl 若誤用該套件，會把 glibc 系統函式庫帶入 musl target。
  - `Cargo.toml` 因此只針對 `target_os=linux` 且 `target_env=musl` 啟用 `dbus/vendored`，由 `libdbus-sys` 以 musl C toolchain 編譯內含的 libdbus。
- README 更新：
  - 新增 release workflow badge。
  - 補上 CHANGELOG 維護格式、版本同步、annotated tag 指令、六平台封裝及 prerelease 規則。
- 官方查核來源：
  - GitHub Actions `push.tags` 與 workflow syntax：
    - <https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax>
  - GitHub-hosted runner 與架構標籤：
    - <https://docs.github.com/en/actions/reference/runners/github-hosted-runners>
  - GitHub CLI release 建立與 `--verify-tag`：
    - <https://cli.github.com/manual/gh_release_create>
- 受影響檔案：
  - `.github/workflows/release.yml`
  - `CHANGELOG.md`
  - `Cargo.toml`
  - `Cargo.lock`
  - `README.md`
  - `DEVELOPMENT_LOG.md`
  - `DEBUGGING_NOTES.md`
- 驗證結果：
  - `actionlint 1.7.12` 通過 `ci.yml` 與 `release.yml`；下載的 actionlint 壓縮檔已先依官方 release checksum 驗證。
  - Ruby YAML parser 通過兩個 workflow 的基本語法解析。
  - `cargo tree --target x86_64-unknown-linux-musl -e features -i libdbus-sys` 確認 musl target 已啟用 `libdbus-sys/vendored`。
  - `cargo xtask ci` 通過，包含格式、Clippy 零警告與全部 55 項測試。
  - 本機 `aarch64-apple-darwin` release binary、tar.gz 內容、`adoctl 0.1.0` 版本及 CHANGELOG release notes 擷取均已驗證。
  - 六平台 matrix 與 GitHub Release 建立必須在 workflow 已存在於遠端標籤所指提交後，由實際 tag push 驗證；本次未建立或推送標籤，因此沒有宣稱已完成遠端 release run。

* * *

## GitHub repository 重新命名

- GitHub repository 由 `doggy8088/ado-manager` 重新命名為 `doggy8088/adoctl`，使 repository 名稱與 CLI binary、Cargo package 一致。
- repository 維持私有、owner 與 repository ID 不變，預設分支仍為 `main`。
- 本機 `origin` fetch／push URL 更新為 `https://github.com/doggy8088/adoctl.git`。
- 同步更新 README workflow badge、CHANGELOG compare／release 連結及開發紀錄中的歷史 Actions run URL。
- 本機 workspace 目錄仍為 `/Users/will/projects/ado-manager`；Git repository 重新命名不要求同時搬移本機目錄，避免在進行中的工作階段破壞 workspace 路徑。
- 驗證結果：
  - GitHub repository metadata 回報 `doggy8088/adoctl`，repository ID 維持 `R_kgDOToZB2Q`。
  - `git ls-remote --exit-code origin HEAD` 成功取得遠端 HEAD `6a0f1bcaa2d0946b7b06e4a6535a892c66079350`。
  - 專案檔案不再包含重新命名前的完整 GitHub URL。
