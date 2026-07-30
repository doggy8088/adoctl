# npm 封裝與 Trusted Publishing

本文件說明 `adoctl` 的 npm 封裝架構、首次發布、Trusted Publishing 設定參數、後續版本發布及本機驗證方式。

* * *

## 架構與限制

GitHub repository `doggy8088/adoctl` 目前是私有 repository。公開 npm 套件的安裝者無法匿名下載私有 GitHub Release，因此 npm 套件不採用安裝時連回 GitHub 的 `postinstall` 下載方式。

發布流程會：

1. 從同一版本的私有 GitHub Release 下載六平台壓縮檔與 `SHA256SUMS`。
2. 驗證每個壓縮檔的 SHA-256 及封裝內容。
3. 擷取六平台的 `adoctl` 原生執行檔。
4. 建立 `npm/native/MANIFEST.json`，記錄來源封裝與 binary SHA-256。
5. 將 JavaScript wrapper 與六平台 binary 一起發布成單一 `adoctl` npm 套件。
6. 執行時由 `npm/cli.cjs` 選擇目前平台的 binary，不啟動 shell，也不轉譯 CLI 參數。

這個設計會讓每位 npm 使用者下載全部六個 binary，但不需要公開 GitHub repository、額外公開檔案服務、長效下載 token 或多個平台子套件。

**Trusted Publishing 可用於私有 GitHub repository，但 npm 官方不支援替私有 repository 產生 provenance。** 即使 npm 套件是公開套件，也不會有 provenance attestation。若未來要啟用 provenance，必須先將來源 repository 改為公開，並重新確認 npm 官方當時的限制。

* * *

## 支援平台

| Node.js 平台 | Rust target | npm 內含執行檔 |
| --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | `adoctl` |
| macOS Intel | `x86_64-apple-darwin` | `adoctl` |
| Linux ARM64 GNU | `aarch64-unknown-linux-gnu` | `adoctl` |
| Linux x64 GNU | `x86_64-unknown-linux-gnu` | `adoctl` |
| Linux x64 musl | `x86_64-unknown-linux-musl` | `adoctl` |
| Windows x64 MSVC | `x86_64-pc-windows-msvc` | `adoctl.exe` |

Linux 會依 Node.js runtime report 判斷 GNU 或 musl。若環境無法正確回報，可明確設定：

```sh
ADOCTL_LIBC=gnu adoctl --version
ADOCTL_LIBC=musl adoctl --version
```

`ADOCTL_LIBC` 只接受 `gnu` 或 `musl`。Linux ARM64 目前只提供 GNU binary；Windows ARM64、Linux ARM64 musl 及其他未列出的組合會回報明確錯誤。

* * *

## 專案固定參數

| 參數 | 設定值 | 說明 |
| --- | --- | --- |
| npm registry | `https://registry.npmjs.org/` | 不發布到 GitHub Packages。 |
| npm 套件名稱 | `adoctl` | 未加 scope 的公開套件。 |
| npm access | `public` | 首次發布也必須明確使用 `--access public`。 |
| npm 初始版本 | `0.1.0` | 必須與 `Cargo.toml`、`package-lock.json` 及 GitHub tag `v0.1.0` 一致。 |
| npm package owner | `willh` | 首次發布時目前登入的 npm 帳號。 |
| GitHub organization 或 user | `doggy8088` | Trusted Publisher 的 repository owner。 |
| GitHub repository | `adoctl` | 只填 repository 名稱，不含 owner。 |
| Workflow filename | `release.yml` | 只填檔名，不填 `.github/workflows/`。大小寫必須完全一致。 |
| GitHub environment | 留空 | 目前 workflow 沒有設定 deployment environment。 |
| Allowed action | `npm publish` | 對應 `--allow-publish`；目前不使用 staged publishing。 |
| Workflow runner | `ubuntu-24.04` | GitHub-hosted runner；self-hosted runner 不支援 npm Trusted Publishing。 |
| Workflow OIDC permission | `id-token: write` | 只授予 `publish-npm` job。 |
| Workflow repository permission | `contents: read` | 取出原始碼及下載私有 GitHub Release。 |
| Node.js | `24.18.0` | 發布 workflow 固定版本。npm 官方最低要求仍應以當時文件為準。 |
| npm CLI | `12.0.2` | 發布 workflow 固定版本；`npm trust` 至少需要 npm 11.15.0。 |
| npm token secret | 不設定 | Trusted Publishing 使用短效 OIDC，不使用 `NPM_TOKEN`。 |
| 穩定版本 dist-tag | `latest` | `npm publish` 預設值。 |
| 預發布版本 dist-tag | `next` | Cargo 版本含 `-` 時由 workflow 自動加入 `--tag next`。 |

`package.json` 的 `repository.url` 固定為：

```text
git+https://github.com/doggy8088/adoctl.git
```

此值必須與 Trusted Publisher 指向的 GitHub repository 完全一致。

* * *

## 首次本機封裝與安裝

必要工具：

- Node.js 20 以上；設定 Trusted Publishing 時建議直接使用專案固定的 Node.js 24。
- npm 11.15.0 以上；下列流程使用 npm 12.0.2。
- GitHub CLI，且 `gh auth status` 必須能讀取私有 repository `doggy8088/adoctl`。
- `tar`、`unzip` 與 `shasum`。

先安裝鎖定的 npm CLI 與相依套件：

```sh
npm install --global npm@12.0.2
npm ci --ignore-scripts
npm test
```

下載已發布的 GitHub Release 資產：

```sh
release_assets_dir="$(mktemp -d -t adoctl-npm-release)"
gh release download v0.1.0 \
  --repo doggy8088/adoctl \
  --dir "$release_assets_dir"
```

準備六平台 binary、驗證 checksum 並建立 npm tarball：

```sh
npm run npm:prepare -- --artifacts "$release_assets_dir"
npm run npm:verify
npm pack --ignore-scripts
```

也可使用 Makefile：

```sh
gh release download v0.1.0 \
  --repo doggy8088/adoctl \
  --dir npm/.artifacts
make npm-install-local NPM_ARTIFACTS_DIR=npm/.artifacts
```

`make npm-install-local` 會建立 `adoctl-0.1.0.tgz`，再以 npm 安裝到 `~/.local/bin/adoctl`。

驗證本機安裝：

```sh
~/.local/bin/adoctl --version
~/.local/bin/adoctl --help
```

若只想隔離測試，不寫入 `~/.local`：

```sh
npm_test_prefix="$(mktemp -d -t adoctl-npm-prefix)"
npm install \
  --global \
  --prefix "$npm_test_prefix" \
  ./adoctl-0.1.0.tgz
"$npm_test_prefix/bin/adoctl" --version
```

產生的 `npm/native/`、`npm/.artifacts/` 與 `adoctl-*.tgz` 都已加入 `.gitignore`，不可提交。

* * *

## 首次發布至 npm registry

**npm Trusted Publisher 只能設定在已經存在的套件上。** 因此 `adoctl` 第一次必須由具有套件名稱建立權限的 npm 帳號在本機發布；完成後才能建立 OIDC 信任關係。

先確認名稱仍未被占用、登入帳號及封裝內容：

```sh
npm view adoctl
npm whoami
npm publish --dry-run --access public
```

名稱尚未存在時，`npm view adoctl` 應回傳 `E404`。目前預定的首次 owner 是 `willh`。

正式建立初始版本：

```sh
npm publish --access public
```

若 npm 帳號或 registry 政策要求一次性密碼：

```sh
npm publish --access public --otp <六位數一次性密碼>
```

發布後核對：

```sh
npm view adoctl@0.1.0 \
  name version dist.tarball dist.integrity repository --json
npm install --global adoctl@0.1.0
adoctl --version
```

npm 版本不可覆寫或重複發布。正式執行 `npm publish` 前，必須先完成 tarball 本機安裝測試。

* * *

## 設定 Trusted Publisher

前置條件：

- `adoctl` 已存在於 npm registry。
- 執行者對套件有 write 權限。
- npm 帳號已啟用 2FA。
- npm CLI 版本至少為 11.15.0。
- `.github/workflows/release.yml` 已存在於 GitHub repository。

使用 CLI 設定：

```sh
npm trust github adoctl \
  --file release.yml \
  --repo doggy8088/adoctl \
  --allow-publish
```

這些參數分別代表：

| CLI 參數 | 值 | 必要性 |
| --- | --- | --- |
| `[package]` | `adoctl` | 必要；也可在專案根目錄省略。 |
| `--file` | `release.yml` | 必要；只能填 workflow 檔名。 |
| `--repo` | `doggy8088/adoctl` | 建議明確提供，避免依 repository metadata 推導。 |
| `--allow-publish` | 無值的布林旗標 | 必要；允許 workflow 執行 `npm publish`。 |
| `--env` | 不提供 | workflow 沒有 GitHub environment 時不得填入。 |
| `--allow-stage-publish` | 不提供 | 目前不採 staged publishing。 |
| `--yes` | 選用 | 跳過一般確認提示，但不會跳過必要的 2FA。 |
| `--registry` | 預設官方 registry | 只有使用不同 registry 時才需要提供。 |

驗證設定：

```sh
npm trust list adoctl
```

也可在 npmjs.com 的套件設定頁新增 GitHub Actions Trusted Publisher，欄位必須填入：

- Organization or user：`doggy8088`
- Repository：`adoctl`
- Workflow filename：`release.yml`
- Environment name：留空
- Allowed actions：勾選 `npm publish`

確認第一次 OIDC 發布成功後，到套件的 Settings → Publishing access 選擇「Require two-factor authentication and disallow tokens」。完成後不應建立或保存 `NPM_TOKEN`。

* * *

## 後續版本發布

每次發布前必須同步：

1. `Cargo.toml` 的 `package.version`。
2. `package.json` 的 `version`。
3. `package-lock.json` 根套件的 `version`。
4. `CHANGELOG.md` 的版本與發布日期。

更新 npm 版本可使用：

```sh
npm version 0.2.0 --no-git-tag-version
```

完成程式與文件提交後建立 annotated tag：

```sh
git push origin main
git tag -a v0.2.0 -m "發布 adoctl v0.2.0"
git push origin v0.2.0
```

`.github/workflows/release.yml` 會依序：

1. 驗證 tag、Cargo、npm 與 CHANGELOG 版本一致。
2. 執行 Rust 與 npm 測試。
3. 建立六平台 GitHub Release 資產及 `SHA256SUMS`。
4. 建立 GitHub Release。
5. 以 GitHub OIDC 取得 npm 短效發布權限。
6. 重驗 GitHub Release checksum 與封裝內容。
7. 建立含六平台 binary 的 npm tarball。
8. 執行不含 `NPM_TOKEN` 的 `npm publish`。

* * *

## 官方文件

- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
- [npm trust 命令](https://docs.npmjs.com/cli/v11/commands/npm-trust/)
- [npm provenance](https://docs.npmjs.com/generating-provenance-statements/)
- [GitHub Actions OIDC 權限](https://docs.github.com/en/actions/reference/security/oidc)
