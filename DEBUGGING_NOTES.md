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

* * *

## accessLevel 選項與 REST 版本落差

- 問題症狀：
  - REST schema 的 `AccountLicenseType` 列舉值多於 Azure DevOps 產品介面公開的 access level，不能直接把所有 enum 值平鋪成 `--access-level`。
  - GitHub Enterprise 出現在最新 access level mapping，但專案原本使用的 `7.1-preview.4` 與公開 REST 7.1 schema 都沒有 `gitHubLicenseType` 或 `licensingSource=gitHub`。
  - 官方 PATCH response 是 `UserEntitlementsPatchResponse` wrapper，原實作卻直接反序列化成 `UserEntitlement`。
- 根因：
  - `AccountLicenseType`、`LicensingSource`、`MsdnLicenseType`、`GitHubLicenseType` 與產品 access level 是不同層次；一個公開 access level 可能需要多欄位組合。
  - REST schema 保留內部值、sentinel 與缺乏現行公開語意的相容性值，列舉存在不等於可安全直接指派。
  - GitHub Enterprise 欄位直到官方 `7.2-preview.5` schema 才出現，而且官方仍沒有手動 PATCH GitHub Enterprise 的 request 範例。
- 修正方式：
  - 查詢使用 `AccessLevel`，涵蓋六種公開顯示層級。
  - 更新使用獨立的 `AssignableAccessLevel`，只涵蓋五種具備公開 request mapping 的層級。
  - GitHub Enterprise 僅供 `user list` 過濾；不對 `user set-access` 宣稱無法驗證的能力。
  - 單一使用者 GET／PATCH 採 REST 7.1，並依官方 wrapper 解析成功回應。
- 維護注意事項：
  - 新增 access level 前，必須同時確認產品層級、欄位組合、API version、可寫入性及實際授權驗證方式。
  - `earlyAdopter` 是 Microsoft 內部值；`none` 不是獨立層級；`professional` 查無足夠官方資料說明現行權益，不得自行推定。
  - Visual Studio 與 GitHub Enterprise 權益會由服務驗證；CLI request 成功不能被描述為建立訂閱。
  - 群組規則或外部訂閱可能影響最終有效層級，降低直接指派不保證立即降低有效權益。

* * *

## GitHub Release 與跨平台封裝注意事項

- 觸發條件：
  - 既有 `ci.yml` 的 `push.branches` 只包含 `main`，單獨推送 tag 不會執行該 workflow。
  - release workflow 必須自行執行 `cargo xtask ci`，不可假設標籤所指提交已經通過另一個 workflow。
  - `push.tags: ["v*"]` 是 glob filter，不是語意化版本 regex；因此 workflow 內仍需比對 `GITHUB_REF_NAME` 與 Cargo package version。
- 版本一致性：
  - 發布前必須同時更新 `Cargo.toml`、`Cargo.lock` 與 `CHANGELOG.md`。
  - 標籤格式固定為 `v<package.version>`；任何不一致都應在建立封裝前失敗。
  - `gh release create --verify-tag` 很重要，否則 GitHub CLI 在找不到標籤時可以自動建立標籤，可能讓錯誤版本被發布。
- 權限隔離：
  - 品質檢查與封裝 job 不需要寫入 repository，應維持 `contents: read`。
  - 只有建立 GitHub Release 的 job 需要 `contents: write`。
  - 不應把 release workflow 的寫入權限直接移到既有 pull request CI，避免不受信任的變更取得多餘權限。
- 跨平台 runner：
  - `macos-latest` 目前是 ARM64 runner；Intel 產物必須使用 GitHub 官方的 `macos-15-intel`。
  - ARM64 GNU Linux 產物使用 `ubuntu-24.04-arm` 原生建置，避免額外維護交叉 linker。
  - Windows 使用 PowerShell `Compress-Archive`；Unix runner 使用 `tar`，不要假設所有 runner 具有相同 shell 或封裝命令。
- musl 與 D-Bus：
  - Ubuntu 的 `libdbus-1-dev` 是 GNU／glibc 系統函式庫，不能直接當成 musl target 的連結輸入。
  - `dbus` crate 的 `vendored` feature 會讓 `libdbus-sys` 透過 `cc` 使用目標 C compiler 建置內含 libdbus。
  - musl runner 仍需安裝 `musl-tools`，提供 `musl-gcc`；只啟用 Rust target 並不足以編譯 C 相依套件。
  - 維護 Cargo feature 時，應以 `cargo tree --target x86_64-unknown-linux-musl -e features -i libdbus-sys` 確認 vendored feature 沒有被移除。
- artifact 與 release：
  - matrix job 只能各自存取自己的工作目錄，必須先使用 workflow artifact 集中產物，再由單一 release job 建立 GitHub Release。
  - release 資產本身已是 ZIP 或 tar.gz；workflow artifact 只是 job 間傳遞媒介，不應把 Actions artifact 的下載網址當成永久發布網址。
  - `SHA256SUMS` 必須對最終上傳的壓縮檔計算，不能對封裝前 binary 計算後卻使用相同檔名宣稱可驗證 release asset。
  - CHANGELOG release notes 擷取必須在下一個版本標題或 Markdown reference link definition 前停止，避免把檔尾連結定義誤放進 GitHub Release 說明。

* * *

## GitHub repository 重新命名注意事項

- GitHub 會保留舊 repository URL 的重新導向，但本機 remote、workflow badge、CHANGELOG compare link 與 release link 不應長期依賴重新導向。
- repository 重新命名後，應以新 URL 執行 `git remote set-url origin <新網址>`，再用 `git ls-remote --exit-code origin HEAD` 驗證 fetch 路徑。
- GitHub repository 的重新命名不會自動重新命名本機 checkout 資料夾；兩者沒有 Git 技術上的相依關係。
- 若未來搬移本機資料夾，必須先結束所有依賴舊 workspace 絕對路徑的程序，再由檔案系統層級搬移；不可在 agent 仍以舊 cwd 運作時直接改名。

* * *

## v0.1.0 發布後封裝內容檢查誤判

- 問題症狀：
  - 從 GitHub Release 下載的六個封裝均通過 `SHA256SUMS`，但第一版內容檢查指令仍以結束碼 1 停止。
  - workflow 與 GitHub Release 本身均為成功，壓縮檔也可以正常列出內容。
- 排查過程：
  - 直接以 `tar -tzf` 與 `unzip -Z1` 列出每個封裝，不再只依賴組合腳本的最終結束碼。
  - 確認 Unix 封裝的項目是 `adoctl`、`README.md`、`CHANGELOG.md`；Windows 封裝的項目是 `adoctl.exe`、`README.md`、`CHANGELOG.md`。
  - 原驗證規則使用 `/adoctl$`、`/README.md$` 與 `/CHANGELOG.md$`，錯誤假設項目前方一定存在子目錄。
- 根因：
  - release workflow 刻意把三個檔案放在壓縮檔根目錄，項目名稱不含 `/`；因此原規則不可能比對成功。
  - `pipefail` 不是本次誤判的根因；在查看實際封裝清單後修正先前判斷。
- 修正方式：
  - 改用 `^adoctl$`、`^adoctl\.exe$`、`^README\.md$` 與 `^CHANGELOG\.md$` 精確比對根目錄項目。
  - 額外斷言每個封裝恰好包含三個項目，避免只檢查必要檔案存在卻漏掉非預期內容。
  - 修正後六個封裝內容斷言與 SHA-256 校驗全部通過。
- 另一項本機驗證注意事項：
  - 驗證腳本進入下載用暫存目錄後執行 `git ls-remote origin`，因該目錄不是 Git repository 而失敗。
  - SHA-256 與封裝內容檢查在該步驟前已全部完成；標籤查詢改回專案工作目錄後成功。
- 維護注意事項：
  - 組合多種驗證的 shell 腳本應保留各階段可辨識的輸出，不能把最後一個無關步驟的失敗誤判為先前資產校驗失敗。
  - 封裝內容規則必須與實際 archive layout 一致；若未來改為外層版本目錄，應同步調整測試與 README 安裝指令。

* * *

## repository 公開後由內含 binary 改為薄封裝

- 問題症狀：
  - Rust CLI npm 封裝的常見做法是在 `postinstall` 依平台下載 GitHub Release binary。
  - 實作開始時 `doggy8088/adoctl` 是私有 repository，公開 npm 安裝者沒有 Release 權限，因此初版曾把六平台 binary 全部放入 npm tarball。
  - 實作期間使用者通知 repository 已公開；訊息寫成 `adocli`，但 GitHub metadata 顯示實際 repository 仍是 `doggy8088/adoctl`。
- 根因：
  - 私有 GitHub Release 確實不能供公開 npm 匿名下載，但 repository visibility 已改變，原限制不再成立。
  - 若繼續發布內含六平台 binary 的 17 MB tarball，每位使用者都會下載不需要的平台，與薄封裝目標不符。
- 修正方式：
  - 以 GitHub metadata 查核 `doggy8088/adoctl` 為 `PUBLIC`，並確認不存在 `doggy8088/adocli`。
  - 移除發布時內含六平台 binary、manifest 與資產準備程式。
  - 初版新增 `postinstall`，只下載目前 target 的版本化資產及 `SHA256SUMS`，通過 SHA-256 後才解壓。
  - 發布前以 HEAD request 驗證六個壓縮檔與 `SHA256SUMS` 共七個公開 URL。
- 維護注意事項：
  - repository 若再次改為私有，薄封裝會立即影響新安裝；必須在變更 visibility 前先設計公開 binary hosting。
  - 不可只下載 checksum 卻不比對精確資產檔名；同一份 `SHA256SUMS` 內有六筆記錄。

* * *

## npm 12 預設阻擋 postinstall

- 問題症狀：
  - 薄封裝 tarball 在 npm 11 可完成 `postinstall`，但 npm 12.0.2 顯示 install script 被 `allowScripts` 政策阻擋。
  - package 本身安裝成功，實際執行 `adoctl` 時卻找不到尚未下載的原生 binary。
- 根因：
  - npm 12 預設不執行未獲允許的 dependency install scripts；使用者必須額外提供 `--allow-scripts=adoctl` 才會執行。
  - 把額外旗標當成標準安裝必要條件，會使 `npm install --global adoctl` 產生表面成功、實際不可用的安裝。
- 修正方式：
  - 完全移除 package 的 install lifecycle script。
  - wrapper 在使用者 cache 缺少對應版本／target binary 時，以目前 Node.js 執行 `npm/download.cjs`。
  - 下載器驗證版本化 Release asset 的 SHA-256 後，以原子 rename 放入 cache，再由 wrapper 啟動。
  - cache 預設使用作業系統慣例，可用 `ADOCTL_CACHE_DIR` 覆寫，支援全域 npm package 目錄唯讀的情境。
- 維護注意事項：
  - 第一次執行需要 GitHub Release 網路連線；離線部署應先以 `ADOCTL_CACHE_DIR` 預熱相同版本與 target 的 binary。
  - 第一次下載訊息輸出到 `stderr`，不能污染 `--output json` 的 `stdout`。

* * *

## npm Trusted Publishing 的首次發布循環相依

- 問題症狀：
  - 需求希望第一次就使用 Trusted Publishing，但 npm 的 Trusted Publisher 設定頁與 `npm trust` 都以既有 package 為設定對象。
- 根因：
  - npm 官方 `npm trust` 前置條件明確要求 package 已存在於 registry；OIDC 信任關係不能用來建立一個尚不存在的 package。
- 修正方式：
  - 初始 `adoctl@0.1.0` 先由已登入、具有 2FA 的 npm maintainer 在本機執行一次 `npm publish --access public`。
  - 初始 package 存在後，建立指向 `doggy8088/adoctl` 與 `release.yml` 的 Trusted Publisher，允許 `npm publish`。
  - 後續版本由 `release.yml` 的 `publish-npm` job 使用 `id-token: write` 及 OIDC 發布，不保存 `NPM_TOKEN`。
- 實際結果：
  - `adoctl@0.1.0` 已完成初始發布，並從 Registry 重新安裝及執行成功。
  - `npm trust list adoctl --json` 已確認 GitHub repository、workflow 檔名與發布權限。
  - `0.1.1` 用於首次 OIDC 發布及 provenance 實測，避免把初始人工發布誤記為 Trusted Publishing。
- provenance 限制：
  - Trusted Publishing 與 provenance 是不同能力；本 repository 現已公開，配合公開 npm package、GitHub-hosted runner 與 OIDC，符合 npm 自動 provenance 條件。
  - `adoctl@0.1.1` 完成實際 OIDC 發布後，Registry 已回傳 SLSA v1 provenance。
  - attestation 的 repository、workflow path、tag ref、GitHub-hosted builder 與 Actions invocation ID 均已比對，不能只以 workflow 成功狀態推定 provenance 存在。
- 維護注意事項：
  - Trusted Publisher 的 workflow filename 只填 `release.yml`，不是完整路徑；repository 與檔名大小寫必須精確一致。
  - 若 workflow 新增 GitHub environment，npm Trusted Publisher 的 Environment name 也必須同步更新。
  - 建立 OIDC 信任並驗證成功後，應把 npm Publishing access 改為要求 2FA 並禁止傳統 token。

* * *

## 新增 MIT 授權時的多處 metadata 漂移

- 問題症狀：
  - 專案先前沒有 `LICENSE`；npm metadata 明確使用 `UNLICENSED`，Cargo metadata 則沒有 license 與 authors。
  - Release workflow 只封裝 README 與 CHANGELOG，即使只新增根目錄 `LICENSE`，各平台下載檔仍不會自動包含授權條款。
- 根因：
  - Cargo、npm package、npm lockfile 與 GitHub Release archive 是彼此獨立的發布表面，新增單一檔案不會自動同步其他 metadata。
- 修正方式：
  - 同步更新 Cargo、npm、lockfile、README、CHANGELOG、發布文件及兩種 Release archive。
  - 在 npm 發布前檢查與 Node.js 測試加入 author、license 及授權檔內容驗證。
- 維護注意事項：
  - npm 已發布版本不可覆寫；授權 metadata 變更只能隨新版本發布。
  - 未來變更著作權人或授權條款時，必須同步檢查 Cargo、npm、lockfile、Release workflow 與封裝內容。
