#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const { existsSync, readFileSync } = require('node:fs');
const { join } = require('node:path');
const { ALL_TARGETS, sha256 } = require('./prepare-package.cjs');
const { binaryName, cargoTarget } = require('./platform.cjs');

const PACKAGE_ROOT = join(__dirname, '..');
const NATIVE_DIR = join(__dirname, 'native');

function cargoVersion() {
  const cargoToml = readFileSync(join(PACKAGE_ROOT, 'Cargo.toml'), 'utf8');
  const packageSection = cargoToml.match(/\[package\]([\s\S]*?)(?:\n\[|$)/);
  const version = packageSection?.[1].match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) {
    throw new Error('無法從 Cargo.toml 讀取 package.version。');
  }
  return version;
}

function verifyPackage({ runCurrentBinary = true } = {}) {
  const packageJson = require(join(PACKAGE_ROOT, 'package.json'));
  if (packageJson.name !== 'adoctl') {
    throw new Error('package.json 的套件名稱必須是 adoctl。');
  }
  if (packageJson.version !== cargoVersion()) {
    throw new Error(
      `npm 版本 ${packageJson.version} 與 Cargo 版本 ${cargoVersion()} 不一致。`,
    );
  }
  if (packageJson.repository?.url !== 'git+https://github.com/doggy8088/adoctl.git') {
    throw new Error('package.json 的 repository.url 與 Trusted Publishing repository 不一致。');
  }
  if (packageJson.publishConfig?.registry !== 'https://registry.npmjs.org/') {
    throw new Error('package.json 的 publishConfig.registry 必須是 npm 官方 registry。');
  }
  if (packageJson.publishConfig?.access !== 'public') {
    throw new Error('package.json 的 publishConfig.access 必須是 public。');
  }

  const manifestPath = join(NATIVE_DIR, 'MANIFEST.json');
  if (!existsSync(manifestPath)) {
    throw new Error('找不到 npm/native/MANIFEST.json；請先準備 GitHub Release 資產。');
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  if (manifest.version !== packageJson.version) {
    throw new Error(`原生執行檔 manifest 版本 ${manifest.version} 與 npm 版本不一致。`);
  }
  const actualTargets = Object.keys(manifest.targets ?? {}).sort();
  if (JSON.stringify(actualTargets) !== JSON.stringify([...ALL_TARGETS].sort())) {
    throw new Error('原生執行檔 manifest 未完整涵蓋六個支援 target。');
  }

  for (const target of ALL_TARGETS) {
    const entry = manifest.targets[target];
    const binary = join(NATIVE_DIR, target, binaryName(target));
    if (!existsSync(binary)) {
      throw new Error(`找不到 ${target} 的原生執行檔。`);
    }
    const actualChecksum = sha256(binary);
    if (actualChecksum !== entry.binarySha256) {
      throw new Error(`${target} 原生執行檔的 SHA-256 與 manifest 不一致。`);
    }
  }

  if (runCurrentBinary) {
    const target = cargoTarget();
    const binary = join(NATIVE_DIR, target, binaryName(target));
    const result = spawnSync(binary, ['--version'], { encoding: 'utf8' });
    if (result.error) throw result.error;
    if (result.status !== 0) {
      throw new Error(`目前平台的 adoctl --version 失敗，結束碼為 ${result.status}。`);
    }
    const expected = `adoctl ${packageJson.version}`;
    if (result.stdout.trim() !== expected) {
      throw new Error(`adoctl 版本不一致：預期 ${expected}，實際 ${result.stdout.trim()}。`);
    }
  }

  return manifest;
}

function main() {
  const manifest = verifyPackage();
  console.log(`npm 發布檢查通過：adoctl ${manifest.version}，共 ${ALL_TARGETS.length} 個 target。`);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}

module.exports = {
  cargoVersion,
  verifyPackage,
};
