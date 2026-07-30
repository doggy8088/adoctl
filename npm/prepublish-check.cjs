#!/usr/bin/env node
'use strict';

const { readFileSync } = require('node:fs');
const { request } = require('node:https');
const { join } = require('node:path');
const { URL } = require('node:url');
const { artifactName, releaseBaseUrl } = require('./download.cjs');
const { TARGETS } = require('./platform.cjs');

const PACKAGE_ROOT = join(__dirname, '..');
const MAX_REDIRECTS = 5;

function cargoVersion() {
  const cargoToml = readFileSync(join(PACKAGE_ROOT, 'Cargo.toml'), 'utf8');
  const packageSection = cargoToml.match(/\[package\]([\s\S]*?)(?:\n\[|$)/);
  const version = packageSection?.[1].match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) {
    throw new Error('無法從 Cargo.toml 讀取 package.version。');
  }
  return version;
}

function verifyMetadata() {
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
  if (packageJson.author !== 'Will 保哥') {
    throw new Error('package.json 的 author 必須是 Will 保哥。');
  }
  if (packageJson.license !== 'MIT') {
    throw new Error('package.json 的 license 必須是 MIT。');
  }
  const licenseText = readFileSync(join(PACKAGE_ROOT, 'LICENSE'), 'utf8');
  if (!licenseText.includes('MIT License') || !licenseText.includes('Will 保哥')) {
    throw new Error('LICENSE 必須是以 Will 保哥為著作權人的 MIT License。');
  }
  if (packageJson.scripts?.postinstall) {
    throw new Error('npm 12 預設阻擋 install scripts；package.json 不應設定 postinstall。');
  }
  return packageJson.version;
}

function expectedReleaseUrls(version) {
  const targets = [...new Set(Object.values(TARGETS))].sort();
  const baseUrl = releaseBaseUrl(version);
  return [
    ...targets.map((target) => `${baseUrl}/${artifactName(target, version)}`),
    `${baseUrl}/SHA256SUMS`,
  ];
}

function checkUrl(url, redirectsRemaining = MAX_REDIRECTS) {
  return new Promise((resolve) => {
    const req = request(
      url,
      {
        method: 'HEAD',
        headers: { 'User-Agent': 'adoctl-npm-prepublish' },
      },
      (response) => {
        const { statusCode, headers } = response;
        response.resume();

        if (
          statusCode >= 300 &&
          statusCode < 400 &&
          headers.location &&
          redirectsRemaining > 0
        ) {
          const nextUrl = new URL(headers.location, url).toString();
          checkUrl(nextUrl, redirectsRemaining - 1).then((result) =>
            resolve({ ...result, url }),
          );
          return;
        }

        resolve({
          url,
          ok: statusCode >= 200 && statusCode < 300,
          statusCode,
        });
      },
    );
    req.setTimeout(30_000, () => req.destroy(new Error(`檢查逾時：${url}`)));
    req.on('error', (error) => resolve({ url, ok: false, error: error.message }));
    req.end();
  });
}

async function verifyReleaseAssets({
  version = verifyMetadata(),
  check = checkUrl,
  retries = Number.parseInt(process.env.ADOCTL_RELEASE_ASSET_RETRIES ?? '1', 10),
  retryDelayMs = Number.parseInt(
    process.env.ADOCTL_RELEASE_ASSET_RETRY_DELAY_MS ?? '1000',
    10,
  ),
} = {}) {
  const urls = expectedReleaseUrls(version);
  let failures = [];

  for (let attempt = 1; attempt <= retries; attempt += 1) {
    const results = await Promise.all(urls.map((url) => check(url)));
    failures = results.filter((result) => !result.ok);
    if (failures.length === 0) return urls;
    if (attempt < retries) {
      await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
    }
  }

  const details = failures.map((failure) => {
    const reason = failure.statusCode ? `HTTP ${failure.statusCode}` : failure.error;
    return `- ${failure.url}：${reason}`;
  });
  throw new Error(
    [`GitHub Release v${version} 缺少可下載的 npm 原生資產：`, ...details].join('\n'),
  );
}

async function main() {
  const version = verifyMetadata();
  const urls = await verifyReleaseAssets({ version });
  console.log(`npm 發布檢查通過：adoctl ${version}，共 ${urls.length} 個 Release 資產。`);
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}

module.exports = {
  checkUrl,
  cargoVersion,
  expectedReleaseUrls,
  verifyMetadata,
  verifyReleaseAssets,
};
