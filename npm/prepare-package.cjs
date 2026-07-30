#!/usr/bin/env node
'use strict';

const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} = require('node:fs');
const { tmpdir } = require('node:os');
const { basename, join, resolve } = require('node:path');
const { TARGETS, binaryName } = require('./platform.cjs');

const PACKAGE_ROOT = join(__dirname, '..');
const DEFAULT_NATIVE_DIR = join(__dirname, 'native');
const ALL_TARGETS = Object.freeze([...new Set(Object.values(TARGETS))].sort());

function packageVersion() {
  return require(join(PACKAGE_ROOT, 'package.json')).version;
}

function archiveName(version, target) {
  const extension = target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar.gz';
  return `adoctl-v${version}-${target}.${extension}`;
}

function sha256(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function parseChecksums(text) {
  const checksums = new Map();
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    const match = line.match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (!match) {
      throw new Error(`SHA256SUMS 格式無效：${rawLine}`);
    }
    checksums.set(match[2], match[1].toLowerCase());
  }
  return checksums;
}

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const detail = result.stderr.trim() || result.stdout.trim();
    throw new Error(`${command} 執行失敗${detail ? `：${detail}` : '。'}`);
  }
  return result.stdout;
}

function archiveEntries(archivePath, target) {
  const output =
    target === 'x86_64-pc-windows-msvc'
      ? run('unzip', ['-Z1', archivePath])
      : run('tar', ['-tzf', archivePath]);
  return output
    .split(/\r?\n/)
    .map((entry) => entry.replace(/^\.\//, '').trim())
    .filter(Boolean);
}

function validateArchiveEntries(entries, target) {
  const expected = [binaryName(target), 'CHANGELOG.md', 'README.md'].sort();
  const actual = [...entries].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${target} 封裝內容不符預期：實際為 ${actual.join('、') || '空封裝'}。`,
    );
  }
}

function extractBinary(archivePath, target, destination) {
  const temporaryDir = mkdtempSync(join(tmpdir(), 'adoctl-npm-extract-'));
  const name = binaryName(target);
  try {
    if (target === 'x86_64-pc-windows-msvc') {
      run('unzip', ['-jo', archivePath, name, '-d', temporaryDir]);
    } else {
      run('tar', ['-xzf', archivePath, '-C', temporaryDir, name]);
    }

    const extracted = join(temporaryDir, name);
    if (!existsSync(extracted)) {
      throw new Error(`${basename(archivePath)} 不含 ${name}。`);
    }
    mkdirSync(destination, { recursive: true });
    const output = join(destination, name);
    copyFileSync(extracted, output);
    chmodSync(output, 0o755);
    return output;
  } finally {
    rmSync(temporaryDir, { recursive: true, force: true });
  }
}

function preparePackage({
  artifactsDir,
  nativeDir = DEFAULT_NATIVE_DIR,
  version = packageVersion(),
} = {}) {
  if (!artifactsDir) {
    throw new Error('缺少發布資產目錄；請使用 --artifacts <目錄>。');
  }

  const checksumFile = join(artifactsDir, 'SHA256SUMS');
  if (!existsSync(checksumFile)) {
    throw new Error(`找不到校驗檔：${checksumFile}`);
  }
  const checksums = parseChecksums(readFileSync(checksumFile, 'utf8'));
  const expectedArchives = ALL_TARGETS.map((target) => archiveName(version, target));
  const checksumNames = [...checksums.keys()].sort();
  if (JSON.stringify(checksumNames) !== JSON.stringify([...expectedArchives].sort())) {
    throw new Error('SHA256SUMS 的封裝清單與六個支援 target 不一致。');
  }

  rmSync(nativeDir, { recursive: true, force: true });
  mkdirSync(nativeDir, { recursive: true });

  const manifest = {
    schemaVersion: 1,
    package: 'adoctl',
    version,
    targets: {},
  };

  try {
    for (const target of ALL_TARGETS) {
      const archive = archiveName(version, target);
      const archivePath = join(artifactsDir, archive);
      if (!existsSync(archivePath)) {
        throw new Error(`找不到發布資產：${archive}`);
      }

      const expectedChecksum = checksums.get(archive);
      const actualChecksum = sha256(archivePath);
      if (actualChecksum !== expectedChecksum) {
        throw new Error(
          `${archive} 的 SHA-256 不一致：預期 ${expectedChecksum}，實際 ${actualChecksum}。`,
        );
      }

      const entries = archiveEntries(archivePath, target);
      validateArchiveEntries(entries, target);
      const output = extractBinary(archivePath, target, join(nativeDir, target));
      manifest.targets[target] = {
        archive,
        archiveSha256: actualChecksum,
        binary: binaryName(target),
        binarySha256: sha256(output),
      };
    }

    writeFileSync(join(nativeDir, 'MANIFEST.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  } catch (error) {
    rmSync(nativeDir, { recursive: true, force: true });
    throw error;
  }

  return manifest;
}

function artifactsDirectoryFromArgs(argv = process.argv.slice(2)) {
  const flagIndex = argv.indexOf('--artifacts');
  if (flagIndex >= 0 && argv[flagIndex + 1]) {
    return resolve(argv[flagIndex + 1]);
  }
  if (process.env.ADOCTL_NPM_ARTIFACTS_DIR) {
    return resolve(process.env.ADOCTL_NPM_ARTIFACTS_DIR);
  }
  throw new Error('缺少 --artifacts <目錄> 或 ADOCTL_NPM_ARTIFACTS_DIR。');
}

function main() {
  const artifactsDir = artifactsDirectoryFromArgs();
  const manifest = preparePackage({ artifactsDir });
  console.log(`已準備 adoctl ${manifest.version} 的 ${ALL_TARGETS.length} 個 npm 原生執行檔。`);
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
  ALL_TARGETS,
  archiveEntries,
  archiveName,
  artifactsDirectoryFromArgs,
  parseChecksums,
  preparePackage,
  sha256,
  validateArchiveEntries,
};
