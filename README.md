# adoctl

[![持續整合](https://github.com/doggy8088/ado-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/doggy8088/ado-manager/actions/workflows/ci.yml)

`adoctl` 是以 Rust 開發的跨平台 Azure DevOps 管理 CLI，目標是讓常見的使用者、授權與專案成員管理工作可以用一致、可測試、可自動化的方式執行。

所有 CLI 說明、錯誤與互動訊息皆以繁體中文撰寫。

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

## 登入

PAT 模式會引導使用者建立 PAT，接收貼入的 token，驗證後保存到作業系統憑證庫：

```sh
adoctl --org my-org login --method pat --open-browser
```

Azure CLI 模式會使用本機 `az login` 狀態：

```sh
az login
adoctl --org my-org login --method azure-cli
```

OAuth device code 模式需要 Microsoft Entra public client id：

```sh
adoctl --org my-org login --method device-code --device-client-id <client-id>
```

CI/CD 或無互動環境可使用環境變數：

```sh
export ADOCTL_ORG=my-org
export ADOCTL_PAT=<pat>
adoctl user list --output json
```

若環境已經有 Azure DevOps Extension 慣用的 `AZURE_DEVOPS_EXT_PAT`，`adoctl` 也會自動使用；同時存在時，`--pat` / `ADOCTL_PAT` 優先。

## 常用命令

```sh
adoctl --org my-org user list
adoctl --org my-org user list --access-level basic --search will
adoctl --org my-org user get --upn user@example.com --include-projects
adoctl --org my-org user set-access --upn user@example.com --access-level stakeholder
adoctl --org my-org project list
adoctl --org my-org project list --state well-formed --search platform
adoctl --org my-org project add-user --project MyProject --upn user@example.com
adoctl --org my-org project remove-user --project MyProject --upn user@example.com --group Contributors
adoctl --org miniasp pool list
adoctl --org miniasp pool list --pool-type automation
adoctl --org miniasp pool agents --pool "Default"
adoctl --org miniasp pool jobs --pool "Default"
```

`--pool` 可接受代理程式集區名稱或數字 Id。`pool list --pool-type` 支援 Azure DevOps API 定義的 `automation` 與 `deployment`。

`pool jobs` 會使用 continuation token 讀取 API 提供的所有頁面。可取得的歷史範圍仍受 Azure DevOps 服務端保留政策限制，不能視為永久工作封存。

## 除錯模式

當 API 回應與預期不一致時，可加上 `--debug` 將較詳細的診斷資訊輸出到 `stderr`；不會影響 `stdout` 的 table / JSON 結果：

```sh
adoctl --org my-org --debug user list
cargo run -- --debug user list
```

目前除錯資訊會包含：認證來源（不含 token 值）、HTTP request URL、HTTP status、JSON 回應摘要與分頁資訊。

## accessLevel

第一版只允許以下安全集合：

| CLI 值 | Azure DevOps API 值 |
| --- | --- |
| `stakeholder` | `stakeholder` |
| `basic` | `express` |
| `basic-test-plans` | `advanced` |

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
```

`make test` 與 `make ci` 都沿用 `cargo xtask` 的品質檢查流程；`make test-unit` 只執行 workspace 測試。

`make install` 會建置並安裝至 `~/.local/bin/adoctl`。若 shell 找不到 `adoctl`，請確認 `~/.local/bin` 已加入 `PATH`。

GitHub Actions 會在下列情況執行相同的 `cargo xtask ci` 品質檢查：

- 推送至 `main`。
- 建立或更新 pull request。
- 從 GitHub Actions 頁面手動觸發。

CI 使用 Ubuntu runner 與 Rust stable toolchain，依序檢查格式、執行 Clippy 並跑完全部 workspace 測試。

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
