# DEBUGGING_NOTES.md

## 初始開發注意事項

- Azure DevOps Member Entitlement Management、Graph、Projects API 分散在不同 host：`vsaex.dev.azure.com`、`vssps.dev.azure.com`、`dev.azure.com`。程式碼以 `AdoService` 區分服務 base URL，測試可用單一 base URL override。
- Member Entitlement Management `userentitlements` list API 不應假設單次請求會回傳全部成員；`user list` 必須使用 `top` / `skip` 分頁取完後再做本機 filter。
- Azure DevOps 的 Basic 授權在部分 API 中會以 `express` 表示，Basic + Test Plans 會以 `advanced` 表示，因此 CLI 值與 API 值必須集中轉換。
- PAT 不可由 CLI 自動取得；實作與文件都必須明確提醒使用者手動建立並貼入。
- PAT 建立網址需要帶入 organization path；正確格式是 `https://dev.azure.com/{org}/_usersSettings/tokens`，不可使用沒有 organization 的 `https://dev.azure.com/_usersSettings/tokens`。
- 非互動 PAT 可來自 `ADOCTL_PAT` 或 `AZURE_DEVOPS_EXT_PAT`；為避免工具專屬設定被外部工具覆蓋，同時存在時需優先使用 `ADOCTL_PAT`。
- `keyring` crate 預設不啟用平台原生後端；若未指定 feature，macOS 會退回 mock store，造成 `adoctl login` 在同一行程看似成功，但下一次執行讀不到憑證。必須啟用 `apple-native`、`windows-native` 與 Linux 持久化後端。
- OAuth device code 需要 Microsoft Entra public client id；若使用者沒有 app registration，必須回報明確錯誤，不可假裝已登入成功。
- `cargo xtask` 不是 Cargo 內建命令；專案需要 `.cargo/config.toml` alias 將 `cargo xtask ...` 轉成 `cargo run --package xtask -- ...`。
- 對外 CLI 子命令名稱要以 help 與文件為準；若 `clap` variant 保留複數內部名稱，必須明確指定 `#[command(name = "...")]`，避免 variant 名稱外洩成使用者看到的命令。

## `cargo run user list` 顯示空結果排查

- 問題症狀：實際 organization 明明有使用者，但 `cargo run user list --output json` 回傳 `[]`，table 模式則顯示沒有符合條件的使用者。
- 初步懷疑：
  - 認證沒有生效。
  - API 只回傳第一頁或 filter 被意外套用。
  - `serde` 型別與 Azure DevOps 真實 response shape 不一致。
- 排查步驟：
  - 先確認 CLI 沒有報 `NotLoggedIn`，代表認證流程大致正常。
  - 再加上 `--output json` 檢查結果，發現是空陣列而非錯誤訊息，表示 request 有成功完成，但 list payload 可能被反序列化成空集合。
  - 補上 `--debug` 後，可從 `stderr` 看到 HTTP JSON 摘要與 collection key，快速判斷 top-level 是否為 `items`、`members` 或其他欄位。
- 根因：`UserEntitlementList` 只讀 `members`，但 Azure DevOps `userentitlements` list API 實際回傳 `items`，導致 `serde` 套用 `#[serde(default)]` 後把使用者清單默默吃成空 `Vec`。
- 修正方式：
  - `UserEntitlementList.members` 新增 `alias = "items"` 與 `alias = "value"`。
  - integration test mock 改成 `items`，避免測試繼續掩蓋真實 API 格式。
  - 新增 `--debug` 方便後續排查類似「HTTP 成功但結果空白」的問題。
- 維護注意事項：
  - Azure DevOps 各 API collection key 並不一致，新增新 endpoint 時不要先入為主假設都是 `members` 或 `value`。
  - 除錯輸出只能記錄 request/response 摘要，不可印出 PAT、access token、refresh token 或 Authorization header。

## `project list` 實作注意事項

- Azure DevOps Projects list API 與 `userentitlements` 不同，分頁資訊可能放在 response header `x-ms-continuationtoken`，不是 body 內的 `count` 或固定 `skip` 規則。
- 若新增需要讀 header 的 API，不要在 command 層直接重建 HTTP client；應優先擴充 `AdoClient`，讓認證、debug log、錯誤處理與 JSON 解析保持一致。
- `project list` 的 `--state` 應使用集中 enum 做 CLI/API mapping，避免在多處手寫 `wellFormed`、`createPending` 等字串造成拼字不一致。
- `--search` 為 CLI 本機過濾，因此必須在所有頁面讀完後再套用；否則 continuation token 分頁只會搜尋到第一頁。

## Agent skill 建立注意事項

- `.agents/skills/` 在 Pi 的 discovery 規則中，根目錄單一 `.md` 檔不會被當成 skill；必須使用資料夾 + `SKILL.md` 的結構。
- frontmatter 的 `name` 必須與父資料夾完全一致，且只能使用小寫英數與連字號；否則會產生 skill validation 警告。
- 若 skill 說明要提高匹配率，`description` 應具體描述「做什麼」與「何時使用」，不要只寫很泛的敘述。
- 若 skill 有附屬文件，應使用 skill 目錄的相對路徑連結；agent 之後才能正確依 skill 位置載入參考文件。

## 代理程式集區工作清單實作注意事項

- 問題症狀與需求差異：
  - Azure DevOps 公開 REST 7.1 參考文件可直接查到 Pools 與 Agents API，但設定頁顯示的工作清單使用 `pools/{poolId}/jobrequests`，公開 REST 7.1 參考頁沒有列出此操作。
  - `completedRequestCount` 是「每個代理程式最近完成幾筆」的限制，不等同於列出集區所有可取得工作。
- 排查步驟：
  - 先查證 Pools API 的 `poolType` 只接受 `automation` 與 `deployment`。
  - 再查證 Agents List API 的 `includeAssignedRequest` 與 `status` 欄位。
  - 最後比對 Microsoft 官方 Azure DevOps Node API 產生碼，確認 pool-level `getAgentRequests(poolId, top, continuationToken)` 使用 `$top` 與 continuation token，並確認 `TaskAgentJobRequest` 欄位。
- 根因與修正方式：
  - 若只呼叫 `jobrequests` 一次，結果可能受單頁上限截斷，看似「所有工作」但實際只有第一頁。
  - 修正為逐頁讀取 `x-ms-continuationtoken`，直到 header 不再提供 token。
  - 為避免 preview endpoint 或服務版本回傳 collection shape 不同，反序列化同時接受 `{ "value": [...] }` 與直接陣列，但不使用未型別化 JSON 靜默吞掉欄位錯誤。
- 維護注意事項：
  - `jobrequests` 屬 preview / 未列於公開 REST 參考頁的介面，升級 API version 前必須重新核對官方 SDK route、query 與 `TaskAgentJobRequest` 型別。
  - 「所有工作」指 Azure DevOps 目前保留且可透過 API 取得的所有頁面，不代表永久歷史封存；CLI 不應宣稱能繞過服務端保留政策。
  - continuation token 若重複，必須立即回報分頁錯誤，不可持續重送相同頁面。

## Makefile 維護注意事項

- Makefile recipe 必須使用 tab 縮排；若改成空白，GNU Make 與 BSD Make 會回報 `missing separator`。
- `make test` 與 `make ci` 應維持為 `cargo xtask` 的薄包裝，不要在 Makefile 重複列出格式、Clippy 與測試子命令，否則兩處流程容易分歧。
- `make package` 必須先檢查 `TARGET`，避免空值被傳給 xtask 後產生不明確錯誤。
- `make install` 應使用 `cargo install --root ~/.local` 的等效路徑，而不是只指定 `--root ~/.local/bin`；Cargo 會自動在 root 下建立 `bin`，若多加一層會錯誤安裝到 `~/.local/bin/bin`。
- `ARGS`、`TARGET` 與 `LOCAL_INSTALL_ROOT` 只在使用者明確執行對應 target 時轉送；Makefile 不應內嵌 organization、認證資訊或敏感資料。

## GitHub Actions 維護注意事項

- CI 應呼叫 `cargo xtask ci`，不要在 workflow 內重複維護 `cargo fmt`、`cargo clippy` 與 `cargo test` 細節。
- action 應固定至官方 repository 的完整 commit SHA；旁註保留對應語意版本，方便日後稽核與更新。
- Rust stable toolchain 必須明確安裝 `rustfmt` 與 `clippy`，不可假設 runner 預載狀態永遠一致。
- `persist-credentials: false` 適用於只讀 CI；若未來新增需要推送 commit 或 tag 的 release workflow，應另外設計最小必要權限，不可直接擴大現有 CI 權限。
- GitHub Actions 使用 YAML 1.2 語意；若以採用 YAML 1.1 的一般 parser 驗證，頂層 `on` 可能被誤判為布林值，應使用 actionlint 或 GitHub 實際 workflow parser 確認。
- 遠端失敗排查順序：
  - 先用 `gh run list` 找到 workflow run。
  - 再以 `gh run view <run-id> --log-failed` 讀取失敗步驟。
  - 根據實際 log 修正，不可只憑本機成功推定 runner 環境相同。
- 初次 GitHub Actions run `30558515298` 的失敗紀錄：
  - 問題症狀：checkout、Rust stable、`rustfmt` 與 `clippy` 安裝皆成功，但 `cargo xtask ci` 在 Clippy 編譯相依套件時失敗。
  - 根因：`keyring` 的 Linux 原生持久化後端會建置 `libdbus-sys`；GitHub-hosted Ubuntu runner 沒有預先提供 `dbus-1` 開發檔案，導致其 build script 無法透過 `pkg-config` 找到 `dbus-1.pc`。
  - 修正方式：在執行 Rust 品質檢查前，以 apt 安裝 `libdbus-1-dev` 與 `pkg-config`。
  - 平台差異：macOS 本機使用 Apple 原生 credential store，不會建置相同的 Linux D-Bus 路徑；因此 CI 涉及原生系統相依套件時，必須以實際 runner 結果為準。
  - 驗證結果：修正後的 GitHub Actions run `30558727576` 在 Ubuntu runner 通過全部步驟。
