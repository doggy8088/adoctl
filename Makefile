CARGO ?= cargo
NPM ?= npm
LOCAL_INSTALL_ROOT ?= $(HOME)/.local
NPM_INSTALL_PREFIX ?= $(HOME)/.local

.DEFAULT_GOAL := help

.PHONY: help build release check fmt fmt-check clippy test test-unit ci run doc install package package-all npm-test npm-verify-assets npm-pack npm-install-local clean

help: ## 顯示可用的 Make targets
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z0-9_-]+:.*## / {printf "  %-14s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## 建置 workspace 的 debug 版本
	$(CARGO) build --workspace

release: ## 建置 adoctl 的 release 版本
	$(CARGO) build --release --package adoctl

check: ## 檢查 workspace 的所有 targets
	$(CARGO) check --workspace --all-targets

fmt: ## 格式化全部 Rust 程式碼
	$(CARGO) fmt --all

fmt-check: ## 檢查 Rust 程式碼格式
	$(CARGO) fmt --all -- --check

clippy: ## 執行 Clippy 並禁止警告
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test: ## 執行完整本機品質檢查
	$(CARGO) xtask test

test-unit: ## 執行 workspace 測試
	$(CARGO) test --workspace

ci: ## 執行與 CI 相同的檢查
	$(CARGO) xtask ci

run: ## 執行 adoctl，可用 ARGS 傳入參數
	$(CARGO) run --package adoctl -- $(ARGS)

doc: ## 產生 workspace API 文件
	$(CARGO) doc --workspace --no-deps

install: ## 將 adoctl 安裝至 ~/.local/bin
	$(CARGO) install --path . --locked --force --root "$(LOCAL_INSTALL_ROOT)"

package: ## 打包指定平台，需設定 TARGET
	@if [ -z "$(TARGET)" ]; then \
		printf '%s\n' '缺少 TARGET，例如 make package TARGET=x86_64-apple-darwin'; \
		exit 2; \
	fi
	$(CARGO) xtask package --target "$(TARGET)"

package-all: ## 打包全部預設平台
	$(CARGO) xtask package --all-default-targets

npm-test: ## 執行 npm wrapper 測試
	$(NPM) test

npm-verify-assets: ## 驗證 npm metadata 與七個 GitHub Release 資產
	$(NPM) run npm:verify-assets

npm-pack: npm-verify-assets ## 建立可發布的 npm tarball
	$(NPM) pack --ignore-scripts

npm-install-local: npm-pack ## 將 npm tarball 安裝至 ~/.local/bin
	@package_name="$$(node -p "require('./package.json').name")"; \
	package_version="$$(node -p "require('./package.json').version")"; \
	$(NPM) install \
		--global \
		--prefix "$(NPM_INSTALL_PREFIX)" \
		"./$${package_name}-$${package_version}.tgz"; \
	"$(NPM_INSTALL_PREFIX)/bin/adoctl" --version

clean: ## 清除 Cargo 建置產物
	$(CARGO) clean
