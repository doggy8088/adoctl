#!/usr/bin/env node
'use strict';

const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} = require('node:fs');
const { get } = require('node:https');
const { dirname, join } = require('node:path');
const { URL } = require('node:url');
const {
  binaryName,
  cachedBinaryPath,
  cargoTarget,
} = require('./platform.cjs');

const PACKAGE_ROOT = join(__dirname, '..');
const MAX_REDIRECTS = 5;

function packageVersion() {
  return require(join(PACKAGE_ROOT, 'package.json')).version;
}

function artifactName(target, version = packageVersion()) {
  const extension = target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar.gz';
  return `adoctl-v${version}-${target}.${extension}`;
}

function releaseBaseUrl(version = packageVersion()) {
  return `https://github.com/doggy8088/adoctl/releases/download/v${version}`;
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

function verifyChecksum(filePath, checksumText, filename) {
  const expected = parseChecksums(checksumText).get(filename);
  if (!expected) {
    throw new Error(`SHA256SUMS 缺少 ${filename}。`);
  }
  const actual = sha256(filePath);
  if (actual !== expected) {
    throw new Error(`${filename} 的 SHA-256 不一致：預期 ${expected}，實際 ${actual}。`);
  }
}

function download(url, destination, redirectsRemaining = MAX_REDIRECTS) {
  return new Promise((resolve, reject) => {
    const request = get(
      url,
      {
        headers: {
          Accept: 'application/octet-stream',
          'User-Agent': `adoctl-npm/${packageVersion()}`,
        },
      },
      (response) => {
        const { statusCode, headers } = response;
        if (
          statusCode >= 300 &&
          statusCode < 400 &&
          headers.location &&
          redirectsRemaining > 0
        ) {
          response.resume();
          const nextUrl = new URL(headers.location, url).toString();
          download(nextUrl, destination, redirectsRemaining - 1).then(resolve, reject);
          return;
        }
        if (statusCode !== 200) {
          response.resume();
          reject(new Error(`下載失敗，HTTP ${statusCode}：${url}`));
          return;
        }

        const chunks = [];
        response.on('data', (chunk) => chunks.push(chunk));
        response.on('end', () => {
          writeFileSync(destination, Buffer.concat(chunks));
          resolve();
        });
      },
    );
    request.setTimeout(30_000, () => request.destroy(new Error(`下載逾時：${url}`)));
    request.on('error', reject);
  });
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
}

function extractArchive(archivePath, target, destination) {
  mkdirSync(destination, { recursive: true });
  if (target === 'x86_64-pc-windows-msvc') {
    if (process.platform === 'win32') {
      run('powershell', [
        '-NoProfile',
        '-Command',
        'Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force',
        archivePath,
        destination,
      ]);
    } else {
      run('unzip', ['-o', archivePath, '-d', destination]);
    }
  } else {
    run('tar', ['-xzf', archivePath, '-C', destination]);
  }
}

async function installFromRelease({
  target = cargoTarget(),
  version = packageVersion(),
  destination = cachedBinaryPath(version, target),
} = {}) {
  if (existsSync(destination)) return destination;

  const archive = artifactName(target, version);
  const baseUrl = releaseBaseUrl(version);
  const binDir = dirname(destination);
  const temporaryDir = join(binDir, `.tmp-${process.pid}-${Date.now()}`);
  const archivePath = join(temporaryDir, archive);
  const checksumPath = join(temporaryDir, 'SHA256SUMS');
  const extractedDir = join(temporaryDir, 'extracted');
  const partialDestination = `${destination}.partial-${process.pid}`;
  const name = binaryName(target);

  mkdirSync(temporaryDir, { recursive: true });
  try {
    await Promise.all([
      download(`${baseUrl}/${archive}`, archivePath),
      download(`${baseUrl}/SHA256SUMS`, checksumPath),
    ]);
    verifyChecksum(archivePath, readFileSync(checksumPath, 'utf8'), archive);
    extractArchive(archivePath, target, extractedDir);

    const extracted = join(extractedDir, name);
    if (!existsSync(extracted)) {
      throw new Error(`${archive} 不含 ${name}。`);
    }

    mkdirSync(binDir, { recursive: true });
    copyFileSync(extracted, partialDestination);
    chmodSync(partialDestination, 0o755);
    if (existsSync(destination)) {
      rmSync(partialDestination, { force: true });
    } else {
      renameSync(partialDestination, destination);
    }
    return destination;
  } finally {
    rmSync(partialDestination, { force: true });
    rmSync(temporaryDir, { recursive: true, force: true });
  }
}

async function main() {
  const target = cargoTarget();
  await installFromRelease({ target });
  console.error(`已下載 ${target} 的 adoctl ${packageVersion()} 原生執行檔。`);
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`adoctl 原生執行檔下載失敗：${error.message}`);
    process.exit(1);
  });
}

module.exports = {
  artifactName,
  download,
  installFromRelease,
  parseChecksums,
  releaseBaseUrl,
  sha256,
  verifyChecksum,
};
