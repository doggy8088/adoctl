# Rust CLI 詳細檢查清單與實作藍圖

## A. 新增子命令 checklist

1. 在 CLI 定義中新增 subcommand / args
2. 補上：
   - help 文案
   - `override_usage`（若專案有使用）
   - value parser / enum mapping
   - 互斥 / 相依規則
3. 在命令 dispatch 層接線
4. 在執行層補 handler
5. 在輸出層補 table / json renderer
6. 補測試：
   - `--help`
   - 成功案例
   - 缺參數
   - 無效參數
   - JSON 輸出
7. 更新 README 範例

## B. API 命令 checklist

### 請求前

- 確認 base URL / API version / auth 來源
- 定義 request / response 型別
- 決定分頁方式：
  - `top` / `skip`
  - continuation token
  - `nextLink`
  - response header token

### 請求後

- 驗證成功狀態碼範圍
- 對映常見錯誤：
  - 400：參數錯誤
  - 401 / 403：認證 / 權限
  - 404：找不到資源
  - 409：衝突
  - 429：節流
  - 5xx：服務端錯誤
- 若 response shape 不穩定，補 alias 或 normalize
- 若有空結果，確認不是 parser 把資料吃掉

### 最少測試集

- 成功
- 找不到
- 錯誤狀態碼
- request shape 正確
- 多頁資料
- alternate response shape

## C. 輸出設計 checklist

### Table 輸出

- 預設給人看
- 欄位順序固定
- 空結果有友善訊息
- 避免欄位太多導致難讀

### JSON 輸出

- 給自動化使用
- key 命名與型別穩定
- 不要在 JSON 前後插入額外文字
- 除錯訊息一律走 `stderr`

## D. Debug / Logging checklist

建議在以下情境提供可開關的 debug log：

- API 命令沒有報錯，但結果異常為空
- 分頁 / filter / auth 行為不明
- 多環境（local / CI / profile）會導致不同結果

建議記錄：

- 認證來源摘要（不可含 secret）
- request method + URL
- response status
- body 摘要（不可含 secret）
- 分頁資訊
- 本機過濾前後筆數

不可記錄：

- PAT
- access token
- refresh token
- cookie
- Authorization header
- private key

## E. 建議目錄結構

```text
src/
├── main.rs
├── lib.rs
├── cli.rs
├── error.rs
├── output.rs
├── config.rs
├── commands/
│   ├── mod.rs
│   └── <command>.rs
└── api/
    ├── mod.rs
    ├── client.rs
    └── <resource>.rs

tests/
├── cli_help.rs
├── <resource>_client.rs
└── integration_*.rs
```

## F. 新命令設計範例

### CLI 需求先寫成句子

- `mycli item list`：列出項目，可依狀態與關鍵字過濾
- `mycli item get --id <id>`：取得單一項目
- `mycli item create --name <name>`：建立項目

### 接著再映射成參數規則

- `--id` 與 `--name` 是否互斥？
- `--output json` 是否要回傳完整資料？
- 空結果要顯示空陣列還是提示文字？
- `table` 與 `json` 的欄位是否一致？

## G. 發布前 checklist

- `cargo fmt`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run -- --help`
- 驗證至少 2～3 個 README 範例命令
- 檢查所有使用者可見訊息語言是否符合 repo 規範
- 檢查 snapshot / README / debug log 不含敏感資訊
