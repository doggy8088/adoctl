# AGENTS.md

## 專案定位

`adoctl` 是以 Rust 開發的 Azure DevOps 管理 CLI。所有使用者可見訊息、`--help`、錯誤訊息、README 範例與互動提示都必須使用繁體中文。

## 技術原則

- CLI 使用 `clap` 定義命令與參數。
- HTTP 使用 `reqwest` + `rustls`，避免平台 OpenSSL 差異。
- 非同步執行環境使用 `tokio`。
- Azure DevOps API request / response 必須以 `serde` 型別描述。
- 錯誤必須明確回報，不可用寬鬆 catch 或靜默 fallback 掩蓋問題。
- Token、PAT、refresh token 不可寫入 log、測試 snapshot、文件範例或錯誤訊息。

## 命令慣例

- 使用者參數優先使用 `--upn <email>`，並支援 `--id <id>`。
- `--upn` 與 `--id` 互斥，且至少需要一個。
- 專案成員管理預設群組為 `Contributors`，可用 `--group` 覆寫。
- 預設輸出為 table，`--output json` 必須保持穩定，供自動化串接。

## 認證策略

第一版支援：

- PAT：`adoctl login --method pat`
- Azure CLI token：`adoctl login --method azure-cli`
- OAuth device code：`adoctl login --method device-code`

PAT 不能由 CLI 靜默產生；CLI 只能引導使用者開啟 PAT 建立頁、提示必要 scope、接收使用者貼入的 PAT，並驗證後保存。
非互動情境可使用 `ADOCTL_PAT` 或 Azure DevOps Extension 慣用的 `AZURE_DEVOPS_EXT_PAT`；同時存在時，以 `ADOCTL_PAT` 為優先。

## 測試策略

- 參數解析與 help 訊息需有測試。
- accessLevel alias 與 Azure DevOps API enum mapping 需有測試。
- Azure DevOps API client 需使用 mock server 測試成功、找不到、錯誤狀態與 request shape。
- 認證與 credential store 需用 mock / memory store 測試，不可依賴真實 keychain。

## 開發紀錄要求

每次重要變更都必須留下完整開發紀錄，包含：

- 技術決策與理由。
- 受影響的模組與 API。
- 測試策略與驗證結果。
- 除錯過程、問題症狀、根因、修正方式。
- 踩雷心得與未來維護注意事項。

實際紀錄位置：

- `DEVELOPMENT_LOG.md`：記錄開發決策、技術細節、API 行為觀察、測試策略調整。
- `DEBUGGING_NOTES.md`：記錄除錯過程、問題症狀、排查步驟、根因、修正方式與心得。

不要在紀錄中貼上任何 token、PAT、refresh token、cookie 或其他敏感資訊。

## 本機工作命令

```sh
cargo xtask test
cargo xtask package --target <target-triple>
cargo xtask package --all-default-targets
```
