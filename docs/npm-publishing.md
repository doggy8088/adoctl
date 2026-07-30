# npm 封裝與 Trusted Publishing

本文件說明 `adoctl` 的 npm wrapper、首次本機部署、registry 初始發布、Trusted Publishing 設定參數及後續版本流程。

* * *

## 封裝架構

`doggy8088/adoctl` 是公開 GitHub repository，GitHub Release 資產可供 npm 安裝者匿名下載。因此 npm 套件維持薄封裝，只包含：

- `npm/cli.cjs`：選擇並執行目前平台的 Rust binary。
- `npm/download.cjs`：第一次執行時下載 GitHub Release 壓縮檔與 `SHA256SUMS`，校驗後保存 binary。
- `npm/platform.cjs`：作業系統、CPU 架構與 Linux libc 對映。
- `README.md` 與 `CHANGELOG.md`。

npm 套件不使用 `preinstall`、`install` 或 `postinstall`，避免 npm 12 的 install script 預設封鎖。第一次執行 `adoctl` 時才下載目前平台的壓縮檔，不會把六平台 binary 全部放進 npm tarball。下載器使用固定版本的 HTTPS URL、最多跟隨五次重新導向，並在解壓前強制驗證 SHA-256。

公開 GitHub repository、公開 npm 套件、GitHub-hosted runner、OIDC Trusted Publishing 與精確的 `repository.url` 符合 npm 自動 provenance 的條件。發布 workflow 不需要加入 `--provenance`，也不使用 `NPM_TOKEN`。

* * *

## 支援平台

| Node.js 平台 | Rust target | GitHub Release 資產 |
| --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | `adoctl-v<版本>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `adoctl-v<版本>-x86_64-apple-darwin.tar.gz` |
| Linux ARM64 GNU | `aarch64-unknown-linux-gnu` | `adoctl-v<版本>-aarch64-unknown-linux-gnu.tar.gz` |
| Linux x64 GNU | `x86_64-unknown-linux-gnu` | `adoctl-v<版本>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux x64 musl | `x86_64-unknown-linux-musl` | `adoctl-v<版本>-x86_64-unknown-linux-musl.tar.gz` |
| Windows x64 MSVC | `x86_64-pc-windows-msvc` | `adoctl-v<版本>-x86_64-pc-windows-msvc.zip` |

Linux 會依 Node.js runtime report 判斷 GNU 或 musl。若環境無法正確回報，可明確設定：

```sh
ADOCTL_LIBC=gnu adoctl --version
ADOCTL_LIBC=musl adoctl --version
```

`ADOCTL_LIBC` 只接受 `gnu` 或 `musl`。Linux ARM64 目前只提供 GNU binary；Windows ARM64、Linux ARM64 musl 及其他未列出的組合會回報明確錯誤。

* * *

## npm 與 GitHub 固定參數

| 參數 | 設定值 | 說明 |
| --- | --- | --- |
| npm registry | `https://registry.npmjs.org/` | 不發布到 GitHub Packages。 |
| npm 套件名稱 | `adoctl` | 未加 scope 的公開套件。 |
| npm access | `public` | 首次發布明確使用 `--access public`。 |
| npm 初始版本 | `0.1.0` | 與 Cargo 版本及 GitHub tag `v0.1.0` 一致。 |
| npm package owner | `willh` | 首次發布時目前登入的 npm 帳號。 |
| GitHub organization 或 user | `doggy8088` | Trusted Publisher 的 repository owner。 |
| GitHub repository | `adoctl` | 只填 repository 名稱，不含 owner。 |
| Workflow filename | `release.yml` | 只填檔名，不填 `.github/workflows/`。 |
| GitHub environment | 留空 | workflow 目前沒有 deployment environment。 |
| Allowed action | `npm publish` | 對應 `--allow-publish`。 |
| Workflow runner | `ubuntu-24.04` | 必須是 GitHub-hosted runner。 |
| Workflow OIDC permission | `id-token: write` | 只授予 `publish-npm` job。 |
| Workflow repository permission | `contents: read` | 取出原始碼。 |
| Node.js | `24.18.0` | 發布 workflow 固定版本。 |
| npm CLI | `12.0.2` | 發布 workflow 固定版本；`npm trust` 至少需要 npm 11.15.0。 |
| npm token secret | 不設定 | 使用 OIDC 短效權限，不建立 `NPM_TOKEN`。 |
| 穩定版本 dist-tag | `latest` | `npm publish` 預設值。 |
| 預發布版本 dist-tag | `next` | 版本含 `-` 時由 workflow 自動加入 `--tag next`。 |

`package.json` 的 `repository.url` 固定為：

```text
git+https://github.com/doggy8088/adoctl.git
```

此值與 Trusted Publisher 指向的公開 GitHub repository 必須完全一致。

* * *

## 首次版本的本機部署

必要工具：

- Node.js 20 以上；Trusted Publishing workflow 使用 Node.js 24。
- npm 11.15.0 以上；專案固定使用 npm 12.0.2。
- `tar`；Windows ZIP 由 PowerShell `Expand-Archive` 解壓。
- 可連線至 `github.com` 與 GitHub Release 下載網域。

安裝 npm 相依套件並測試 wrapper：

```sh
npm install --global npm@12.0.2
npm ci --ignore-scripts
npm test
```

確認七個 Release 資產可匿名下載：

```sh
npm run npm:verify-assets
```

建立薄封裝 tarball：

```sh
npm pack --ignore-scripts
```

隔離安裝並實際觸發第一次執行下載：

```sh
npm_test_prefix="$(mktemp -d -t adoctl-npm-prefix)"
npm install \
  --global \
  --prefix "$npm_test_prefix" \
  ./adoctl-0.1.0.tgz
"$npm_test_prefix/bin/adoctl" --version
"$npm_test_prefix/bin/adoctl" --help
```

也可直接使用 Makefile 安裝至 `~/.local/bin`：

```sh
make npm-install-local
~/.local/bin/adoctl --version
```

binary cache 預設位置：

| 平台 | cache root |
| --- | --- |
| macOS | `~/Library/Caches/adoctl` |
| Linux | `$XDG_CACHE_HOME/adoctl`，未設定時為 `~/.cache/adoctl` |
| Windows | `%LOCALAPPDATA%\adoctl` |

各版本與 target 會放在 `v<版本>/<Rust target>/` 子目錄。可用 `ADOCTL_CACHE_DIR` 完整覆寫 cache root：

```sh
ADOCTL_CACHE_DIR=/opt/adoctl-cache adoctl --version
```

專案根目錄產生的 `adoctl-*.tgz` 已加入 `.gitignore`，不可提交。

* * *

## 首次發布至 npm registry

**npm Trusted Publisher 只能設定在已存在的套件上。** 因此第一次必須由具有套件名稱建立權限的 npm 帳號在本機發布；套件存在後才能建立 OIDC 信任關係。

`adoctl@0.1.0` 已由 npm 帳號 `willh` 完成初始發布。以下命令保留作為新套件首次部署的可重複操作紀錄，不可對既有版本重複執行。

先確認名稱、登入帳號與實際 tarball：

```sh
npm view adoctl
npm whoami
npm publish --dry-run --access public
```

名稱尚未存在時，`npm view adoctl` 應回傳 `E404`。目前預定首次 owner 是 `willh`。

正式建立初始版本：

```sh
npm publish --access public
```

若 npm 要求一次性密碼：

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

npm 版本不可覆寫或重複發布。正式執行 `npm publish` 前，必須先完成 tarball 的本機隔離安裝測試。

* * *

## 設定 Trusted Publisher

目前 `adoctl` 已使用下列參數建立 Trusted Publisher，並以 `npm trust list adoctl --json` 回讀確認。

前置條件：

- `adoctl` 已存在於 npm registry。
- 執行者對 package 有 write 權限。
- npm 帳號已啟用 2FA。
- npm CLI 至少為 11.15.0。
- `.github/workflows/release.yml` 已存在於 GitHub repository。

使用 CLI 設定：

```sh
npm trust github adoctl \
  --file release.yml \
  --repo doggy8088/adoctl \
  --allow-publish
```

| CLI 參數 | 值 | 必要性 |
| --- | --- | --- |
| `[package]` | `adoctl` | 必要；在專案根目錄可省略。 |
| `--file` | `release.yml` | 必要；只能填 workflow 檔名。 |
| `--repo` | `doggy8088/adoctl` | 建議明確提供。 |
| `--allow-publish` | 無值的布林旗標 | 必要；允許 `npm publish`。 |
| `--env` | 不提供 | workflow 沒有 GitHub environment 時不得填入。 |
| `--allow-stage-publish` | 不提供 | 目前不採 staged publishing。 |
| `--yes` | 選用 | 跳過一般確認，不會跳過必要的 2FA。 |
| `--registry` | 預設官方 registry | 使用其他 registry 時才需提供。 |

驗證設定：

```sh
npm trust list adoctl
```

也可在 npmjs.com 的 package settings 新增 GitHub Actions Trusted Publisher：

- Organization or user：`doggy8088`
- Repository：`adoctl`
- Workflow filename：`release.yml`
- Environment name：留空
- Allowed actions：勾選 `npm publish`

第一次 OIDC 發布成功後，到 Settings → Publishing access 選擇「Require two-factor authentication and disallow tokens」。完成後不應建立或保存 `NPM_TOKEN`。

* * *

## 後續版本與 Trusted Publishing

每次發布前同步：

1. `Cargo.toml` 的 `package.version`。
2. `package.json` 的 `version`。
3. `package-lock.json` 根 package 的 `version`。
4. `CHANGELOG.md` 的版本與發布日期。

更新 npm 版本：

```sh
npm version 0.2.0 --no-git-tag-version
```

建立 annotated tag：

```sh
git push origin main
git tag -a v0.2.0 -m "發布 adoctl v0.2.0"
git push origin v0.2.0
```

`.github/workflows/release.yml` 會：

1. 驗證 tag、Cargo、npm 與 CHANGELOG 版本一致。
2. 執行 Rust 與 npm 測試。
3. 建立六平台 GitHub Release 資產與 `SHA256SUMS`。
4. 建立 GitHub Release。
5. 確認七個公開資產可下載。
6. 以 GitHub OIDC 取得 npm 短效權限。
7. 執行 `npm publish`，並由 npm 自動建立 provenance。

* * *

## 官方文件

- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
- [npm trust 命令](https://docs.npmjs.com/cli/v11/commands/npm-trust/)
- [npm provenance](https://docs.npmjs.com/generating-provenance-statements/)
- [GitHub Actions OIDC 權限](https://docs.github.com/en/actions/reference/security/oidc)
