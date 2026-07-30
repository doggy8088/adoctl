# adoctl

[![持續整合](https://github.com/doggy8088/adoctl/actions/workflows/ci.yml/badge.svg)](https://github.com/doggy8088/adoctl/actions/workflows/ci.yml)
[![發布版本](https://github.com/doggy8088/adoctl/actions/workflows/release.yml/badge.svg)](https://github.com/doggy8088/adoctl/actions/workflows/release.yml)

`adoctl` 是以 Rust 開發的跨平台 Azure DevOps 管理 CLI，目標是讓常見的使用者、授權與專案成員管理工作可以用一致、可測試、可自動化的方式執行。

所有 CLI 說明、錯誤與互動訊息皆以繁體中文撰寫。

* * *

## 功能

- `adoctl login`：協助登入 Azure DevOps，支援 PAT、Azure CLI token、OAuth device code。
- `adoctl user list`：列出 organization 內所有使用者，可依 accessLevel 或姓名 / Email 關鍵字過濾。
- `adoctl user get`：取得使用者資訊、授權與可存取專案。
- `adoctl user set-access`：變更使用者 accessLevel。
- `adoctl project list`：列出 organization 內的專案，可依狀態或關鍵字過濾。
- `adoctl project add-user`：將使用者加入專案群組，預設 `Contributors`。
- `adoctl project remove-user`：將使用者從專案群組移除，預設 `Contributors`。
- `adoctl pool list`：列出 organization 內的代理程式集區，可依集區型別過濾。
- `adoctl pool agents`：列出指定集區的代理程式、連線狀態與目前工作。
- `adoctl pool jobs`：列出指定集區在 Azure DevOps 保留範圍內的所有工作要求。

使用者參數以 UPN / Email 為主，也支援 Azure DevOps 使用者 Id：

```sh
adoctl user get --upn user@example.com
adoctl user get --id 00000000-0000-0000-0000-000000000000
```

* * *

## 安裝

### npm registry

需要 Node.js 20 以上：

```sh
npm install --global adoctl
adoctl --version
adoctl --help
```

npm 套件包含 Windows x64、Linux GNU x64、Linux musl x64、Linux GNU ARM64、macOS Intel 與 macOS Apple Silicon 六種原生執行檔。JavaScript wrapper 只負責選擇目前平台的 Rust binary 並原樣轉交參數。

Linux 會自動判斷 GNU 或 musl；必要時可明確指定：

```sh
ADOCTL_LIBC=gnu adoctl --version
ADOCTL_LIBC=musl adoctl --version
```

`ADOCTL_LIBC` 只接受 `gnu` 或 `musl`。尚未提供 Windows ARM64 與 Linux ARM64 musl。

### 從 Rust 原始碼安裝

```sh
make install
```

`make install` 會建置並安裝至 `~/.local/bin/adoctl`。若 shell 找不到 `adoctl`，請確認 `~/.local/bin` 已加入 `PATH`。

npm tarball 的首次本機封裝、隔離安裝、registry 初始發布及 Trusted Publishing 所有參數，請參閱 [npm 封裝與 Trusted Publishing](docs/npm-publishing.md)。

* * *

## 共用選項

基本語法：

```text
adoctl [共用選項] <命令> [命令選項]
```

| 選項 | 用途 |
| --- | --- |
| `--org <組織>` | Azure DevOps organization 名稱或 URL，也可設定 `ADOCTL_ORG`。 |
| `--profile <設定檔>` | 區分不同 organization 或登入身分；預設為 `default`。 |
| `--auth <方式>` | 指定 `pat`、`azure-cli` 或 `device-code`。 |
| `--output table` | 預設的人類可讀表格輸出。 |
| `--output json` | 穩定的 JSON 輸出，適合自動化串接。 |
| `--debug` | 將 HTTP 與分頁診斷寫入 `stderr`，不污染正常輸出。 |
| `--help` | 顯示根命令或指定子命令的繁體中文說明。 |

常用設定方式：

```sh
export ADOCTL_ORG=my-org
adoctl user list

adoctl --org https://dev.azure.com/my-org --output json project list
adoctl user set-access --help
```

* * *

## 命令用法與範例

### `adoctl login`

用途：驗證 Azure DevOps 認證，並視登入方式保存必要設定。

```text
adoctl login [--method <方式>] [選項]
```

PAT 模式會引導建立 PAT、接收使用者貼入的 PAT、驗證後保存到作業系統憑證庫：

```sh
adoctl --org my-org login --method pat --open-browser
adoctl --org my-org login --method pat --no-store
```

不要把 PAT 直接寫進 shell history。CI/CD 或無互動環境應使用環境變數：

```sh
export ADOCTL_ORG=my-org
export ADOCTL_PAT=<pat>
adoctl --output json user list
```

若環境已有 Azure DevOps Extension 慣用的 `AZURE_DEVOPS_EXT_PAT`，`adoctl` 也會使用；同時存在時，`--pat` 或 `ADOCTL_PAT` 優先。

Azure CLI 模式會沿用本機 `az login` 狀態：

```sh
az login
adoctl --org my-org login --method azure-cli
```

OAuth device code 模式需要 Microsoft Entra public client id：

```sh
adoctl --org my-org login \
  --method device-code \
  --device-client-id <client-id> \
  --tenant organizations
```

### `adoctl user list`

用途：列出 organization 使用者，可依 accessLevel 與姓名、UPN 或 Email 過濾。

```text
adoctl user list [--access-level <等級>] [--search <關鍵字>]
```

```sh
adoctl --org my-org user list
adoctl --org my-org user list --search will
adoctl --org my-org user list --access-level basic
adoctl --org my-org --output json user list --access-level visual-studio-subscriber
```

`user list` 會讀完 API 分頁後才套用 `--access-level` 與 `--search`，避免漏掉後續頁面的使用者。

### `adoctl user get`

用途：依 UPN、Email 或使用者 Id 取得單一使用者資訊。

```text
adoctl user get (--upn <email> | --id <id>) [--include-projects]
```

```sh
adoctl --org my-org user get --upn user@example.com
adoctl --org my-org user get \
  --id 00000000-0000-0000-0000-000000000000 \
  --include-projects
adoctl --org my-org --output json user get --upn user@example.com
```

`--upn` 與 `--id` 互斥，而且至少要提供其中一個。

### `adoctl user set-access`

用途：變更單一使用者的 accessLevel。

```text
adoctl user set-access \
  (--upn <email> | --id <id>) \
  --access-level <等級>
```

五種有完整官方直接指派 mapping 的值都有可直接複製的範例：

```sh
# Stakeholder：免費但功能受限
adoctl --org my-org user set-access \
  --upn user@example.com \
  --access-level stakeholder

# Basic：一般開發人員最常用
adoctl --org my-org user set-access \
  --upn user@example.com \
  --access-level basic

# Basic 的官方 API 別名
adoctl --org my-org user set-access \
  --upn user@example.com \
  --access-level express

# Basic + Test Plans：需要完整測試管理功能
adoctl --org my-org user set-access \
  --upn tester@example.com \
  --access-level basic-test-plans

# Basic + Test Plans 的官方 API 別名
adoctl --org my-org user set-access \
  --upn tester@example.com \
  --access-level advanced

# Visual Studio Subscriber：由服務偵測有效訂閱
adoctl --org my-org user set-access \
  --upn subscriber@example.com \
  --access-level visual-studio-subscriber

# Visual Studio Enterprise：需要有效的 Enterprise 訂閱
adoctl --org my-org user set-access \
  --upn enterprise@example.com \
  --access-level visual-studio-enterprise
```

也可使用 Azure DevOps 使用者 Id，並取得 JSON 回應：

```sh
adoctl --org my-org --output json user set-access \
  --id 00000000-0000-0000-0000-000000000000 \
  --access-level basic
```

此命令需要可管理 organization 使用者授權的權限；PAT 或 OAuth token 也必須包含 Member Entitlement Management 寫入權限。Visual Studio 選項不會建立或偽造訂閱，Azure DevOps 仍會驗證使用者的實際授權。

GitHub Enterprise 權益由 Azure DevOps 在使用者登入後自動偵測。REST 7.1 更新契約沒有 `gitHubLicenseType`；7.2-preview.5 schema 雖已加入欄位，官方仍沒有提供 GitHub Enterprise 的 PATCH request 範例。因此 `github-enterprise` 可用於 `user list --access-level` 過濾，但**不能用於 `user set-access`**：

```sh
adoctl --org my-org user list --access-level github-enterprise
```

### `adoctl project list`

用途：列出專案，可依專案狀態及名稱、描述或 Id 搜尋。

```text
adoctl project list \
  [--state <all|new|create-pending|well-formed|deleting>] \
  [--search <關鍵字>]
```

```sh
adoctl --org my-org project list
adoctl --org my-org project list --state well-formed
adoctl --org my-org project list --state well-formed --search platform
adoctl --org my-org --output json project list --state all
```

### `adoctl project add-user`

用途：把使用者加入專案群組。未提供 `--group` 時使用 `Contributors`。

```text
adoctl project add-user \
  --project <專案名稱或 Id> \
  (--upn <email> | --id <id>) \
  [--group <群組名稱或 descriptor>]
```

```sh
adoctl --org my-org project add-user \
  --project MyProject \
  --upn user@example.com

adoctl --org my-org project add-user \
  --project MyProject \
  --id 00000000-0000-0000-0000-000000000000 \
  --group Readers
```

### `adoctl project remove-user`

用途：把使用者從專案群組移除。未提供 `--group` 時使用 `Contributors`。

```text
adoctl project remove-user \
  --project <專案名稱或 Id> \
  (--upn <email> | --id <id>) \
  [--group <群組名稱或 descriptor>]
```

```sh
adoctl --org my-org project remove-user \
  --project MyProject \
  --upn user@example.com

adoctl --org my-org project remove-user \
  --project MyProject \
  --upn user@example.com \
  --group Contributors
```

### `adoctl pool list`

用途：列出 organization 內的代理程式集區，可依集區型別過濾。

```text
adoctl pool list [--pool-type <automation|deployment>]
```

```sh
adoctl --org my-org pool list
adoctl --org my-org pool list --pool-type automation
adoctl --org my-org --output json pool list --pool-type deployment
```

`automation` 是一般建置與部署工作使用的代理程式集區；`deployment` 是部署群組相關集區。

### `adoctl pool agents`

用途：列出指定代理程式集區內的代理程式、啟用狀態、連線狀態與目前工作。

```text
adoctl pool agents --pool <集區名稱或數字 Id>
```

```sh
adoctl --org my-org pool agents --pool "Default"
adoctl --org my-org pool agents --pool 42
adoctl --org my-org --output json pool agents --pool "Linux Pool"
```

### `adoctl pool jobs`

用途：列出指定代理程式集區在 Azure DevOps 保留範圍內可取得的所有工作要求。

```text
adoctl pool jobs --pool <集區名稱或數字 Id>
```

```sh
adoctl --org my-org pool jobs --pool "Default"
adoctl --org my-org pool jobs --pool 42
adoctl --org my-org --output json pool jobs --pool "Linux Pool"
```

`pool jobs` 會使用 continuation token 讀取所有 API 分頁。可取得的歷史範圍仍受 Azure DevOps 服務端保留政策限制，不能視為永久工作封存。

* * *

## accessLevel 完整說明

`adoctl user list --access-level` 支援 Microsoft 官方資料中的六種公開 access level；`user set-access` 只接受其中五種具備公開、可驗證直接指派 mapping 的值：

| CLI 值 | 官方顯示名稱 | API 對照 | CLI 支援範圍 | 使用建議與說明 |
| --- | --- | --- | --- | --- |
| `stakeholder` | Stakeholder | `account` / `stakeholder` | 查詢、直接設定 | 常見適用情境是只需處理工作項目、查詢及儀表板的人員；免費且人數不限，但私有專案的 Repos、Pipelines 與 Test Plans 功能受限。 |
| `basic`，別名 `express` | Basic | `account` / `express` | 查詢、直接設定 | **常用**。Microsoft 明確建議大多數使用者採用 Basic；提供大多數 Azure Boards、Repos、Pipelines 與 Artifacts 功能。每個 organization 前五名 Basic 使用者免費，超過後可能計費。 |
| `basic-test-plans`，別名 `advanced` | Basic + Test Plans | `account` / `advanced` | 查詢、直接設定 | 適用於需要完整測試管理的團隊，包含 Basic 全部功能及 Azure Test Plans 的測試計畫、測試套件與測試案例管理能力；通常需要付費授權。 |
| `visual-studio-subscriber` | Visual Studio Subscriber | `msdn` / `eligible` | 查詢、直接設定 | 有有效訂閱時使用。Azure DevOps 會辨識實際訂閱層級並套用包含的權益。 |
| `visual-studio-enterprise` | Visual Studio Enterprise | `msdn` / `enterprise` | 查詢、直接設定 | 適合有效的 Visual Studio Enterprise 訂閱者，包含 Basic、進階測試及符合訂閱資格的 Microsoft Marketplace 權益。 |
| `github-enterprise` | GitHub Enterprise | `gitHub` / `enterprise` | 僅查詢過濾 | Azure DevOps 會在使用者登入時自動辨識 GitHub Enterprise 授權並提供相當於 Basic 的功能；REST 7.1 不提供可驗證的手動授予契約。 |

**Microsoft 官方建議大多數使用者使用 Basic accessLevel，並加入 Contributors 安全性群組。** 因此本文件只把 Basic 明確標為「常用」；其他列的適用情境依官方功能與授權說明整理，不代表 Microsoft 公布的實際使用率。Access level 控制可使用的產品功能，安全性群組與權限則控制能對特定資源執行哪些操作，兩者不可互相取代。

REST API 的原始 `AccountLicenseType` enum 還列出下列值，但它們不是本 CLI 可安全設定的獨立公開 access level：

| 原始值 | 不列為 CLI 選項的理由 |
| --- | --- |
| `none` | 必須搭配 `licensingSource` 及其他授權型別，供 Visual Studio 或 GitHub Enterprise mapping 使用，不是獨立 access level。 |
| `earlyAdopter` | Microsoft 官方明確標示為僅供 Microsoft 內部使用。 |
| `professional` | REST enum 與 Azure CLI 參數仍列出此值，但目前官方的 UI／程式化 access level 對照表沒有對應項目或公開語意，因此不把它當成可驗證的使用者選項。 |

官方參考：

- [About access levels](https://learn.microsoft.com/en-us/azure/devops/organizations/security/access-levels?view=azure-devops)
- [User Entitlements - Update User Entitlement](https://learn.microsoft.com/en-us/rest/api/azure/devops/memberentitlementmanagement/user-entitlements/update-user-entitlement?view=azure-devops-rest-7.1)
- [Manage paid access for users](https://learn.microsoft.com/en-us/azure/devops/organizations/billing/buy-basic-access-add-users?view=azure-devops)

* * *

## 除錯模式

當 API 回應與預期不一致時，可加上 `--debug` 將較詳細的診斷資訊輸出到 `stderr`；不會影響 `stdout` 的 table / JSON 結果：

```sh
adoctl --org my-org --debug user list
cargo run -- --debug user list
```

目前除錯資訊包含認證來源摘要、HTTP request URL、HTTP status、JSON 回應摘要與分頁資訊；不會輸出 token、PAT、cookie 或 Authorization header。

* * *

## 測試與本機 CI

```sh
cargo xtask test
```

這會執行：

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

也可透過 Makefile 執行常用開發工作：

```sh
make help
make build
make check
make test
make install
make run ARGS="--org miniasp pool list"
make package TARGET=x86_64-apple-darwin
make package-all
make npm-test
make npm-prepare NPM_ARTIFACTS_DIR=npm/.artifacts
make npm-pack
make npm-install-local NPM_ARTIFACTS_DIR=npm/.artifacts
```

`make test` 與 `make ci` 都沿用 `cargo xtask` 的品質檢查流程；`make test-unit` 只執行 workspace 測試。

`make npm-install-local` 會驗證 GitHub Release 資產、建立含六平台 binary 的 npm tarball，再以 npm 安裝至 `~/.local/bin/adoctl`。

GitHub Actions 會在下列情況執行相同的 Rust 與 npm 品質檢查：

- 推送至 `main`。
- 建立或更新 pull request。
- 從 GitHub Actions 頁面手動觸發。

CI 使用 Ubuntu runner、Rust stable、Node.js 24 與 npm 12，依序檢查格式、執行 Clippy、跑完 workspace 測試、npm wrapper 測試及 npm tarball 清單檢查。

* * *

## 打包

指定平台：

```sh
cargo xtask package --target x86_64-apple-darwin
```

預設平台：

```sh
cargo xtask package --all-default-targets
```

預設 target 包含：

- `x86_64-pc-windows-msvc`
- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

跨平台編譯可能需要先安裝對應 Rust target、linker 或 cross compilation toolchain。

* * *

## 版本紀錄與發布

所有重要變更都記錄於 [CHANGELOG.md](CHANGELOG.md)。開發期間先把內容加入「尚未發布」段落；準備發布時，建立對應版本標題並填入發布日期：

```markdown
## [0.2.0] - 2026-08-15

### 新增

- 說明這個版本新增的功能。
```

發布前必須同步更新 `Cargo.toml`、`package.json`、`package-lock.json` 與 CHANGELOG 的版本。標籤固定使用 `v<版本>`，而且必須與 Cargo 及 npm 版本完全一致：

```sh
git push origin main
git tag -a v0.1.0 -m "發布 adoctl v0.1.0"
git push origin v0.1.0
```

推送 `v*` 標籤後，GitHub Actions 會：

1. 驗證標籤、Cargo 版本及 CHANGELOG 版本標題一致。
2. 執行 `cargo xtask ci` 完整品質檢查。
3. 為六個預設 target 建立封裝：
   - Windows x86_64。
   - Linux GNU x86_64。
   - Linux musl x86_64。
   - Linux GNU ARM64。
   - macOS Intel。
   - macOS Apple Silicon。
4. 驗證封裝內的 `adoctl --version`。
5. 產生 `SHA256SUMS`，並使用 CHANGELOG 對應版本內容建立 GitHub Release。
6. 重新驗證 Release checksum 與六平台 binary。
7. 使用 npm Trusted Publishing 的 OIDC 短效權限發布 `adoctl`，不使用 `NPM_TOKEN`。

含預發布識別碼的 Cargo 版本，例如 `0.2.0-beta.1`，應使用 `v0.2.0-beta.1` 標籤；GitHub Release 會自動標示為 prerelease。

repository 目前是私有 repository。npm Trusted Publishing 可以使用，但 npm 官方不會替私有 repository 產生 provenance；公開 npm 套件會包含 wrapper、README、CHANGELOG 與六平台 binary。

首次 npm 版本必須先在本機建立套件，之後才能設定 Trusted Publisher。完整的固定值、CLI 參數、npmjs.com 欄位與初始發布順序請參閱 [npm 封裝與 Trusted Publishing](docs/npm-publishing.md)。
