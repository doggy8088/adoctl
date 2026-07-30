'use strict';

const assert = require('node:assert/strict');
const { mkdtempSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const test = require('node:test');

const {
  artifactName,
  parseChecksums,
  releaseBaseUrl,
  sha256,
  verifyChecksum,
} = require('../../npm/download.cjs');
const {
  binaryName,
  cacheRoot,
  cachedBinaryPath,
  cargoTarget,
  detectLinuxLibc,
  platformKey,
} = require('../../npm/platform.cjs');
const {
  expectedReleaseUrls,
  verifyReleaseAssets,
} = require('../../npm/prepublish-check.cjs');

test('將六種支援平台映射至 Rust target', () => {
  assert.equal(cargoTarget('darwin', 'arm64'), 'aarch64-apple-darwin');
  assert.equal(cargoTarget('darwin', 'x64'), 'x86_64-apple-darwin');
  assert.equal(cargoTarget('linux', 'arm64', 'gnu'), 'aarch64-unknown-linux-gnu');
  assert.equal(cargoTarget('linux', 'x64', 'gnu'), 'x86_64-unknown-linux-gnu');
  assert.equal(cargoTarget('linux', 'x64', 'musl'), 'x86_64-unknown-linux-musl');
  assert.equal(cargoTarget('win32', 'x64'), 'x86_64-pc-windows-msvc');
});

test('辨識 GNU 與 musl libc 並允許明確覆寫', () => {
  assert.equal(detectLinuxLibc(undefined, { glibcVersionRuntime: '2.39' }), 'gnu');
  assert.equal(detectLinuxLibc(undefined, {}), 'musl');
  assert.equal(detectLinuxLibc('MUSL', { glibcVersionRuntime: '2.39' }), 'musl');
  assert.equal(platformKey('linux', 'x64', 'gnu'), 'linux-x64-gnu');
  assert.throws(() => detectLinuxLibc('unknown', {}), /只接受 gnu 或 musl/);
});

test('拒絕未提供封裝的作業系統、架構或 libc', () => {
  assert.throws(() => cargoTarget('linux', 'arm64', 'musl'), /不支援目前平台/);
  assert.throws(() => cargoTarget('win32', 'arm64'), /不支援目前平台/);
  assert.throws(() => cargoTarget('freebsd', 'x64'), /不支援目前平台/);
});

test('依作業系統使用可覆寫的使用者 cache', () => {
  assert.equal(
    cacheRoot({
      env: { ADOCTL_CACHE_DIR: '/tmp/custom-adoctl-cache' },
      platform: 'linux',
      home: '/home/tester',
    }),
    '/tmp/custom-adoctl-cache',
  );
  assert.equal(
    cacheRoot({ env: {}, platform: 'darwin', home: '/Users/tester' }),
    '/Users/tester/Library/Caches/adoctl',
  );
  assert.equal(
    cachedBinaryPath('0.1.0', 'aarch64-apple-darwin', {
      env: { ADOCTL_CACHE_DIR: '/tmp/adoctl-cache' },
      platform: 'darwin',
      home: '/Users/tester',
    }),
    '/tmp/adoctl-cache/v0.1.0/aarch64-apple-darwin/adoctl',
  );
});

test('產生與 GitHub Release 一致的封裝名稱及網址', () => {
  assert.equal(
    artifactName('aarch64-apple-darwin', '0.1.0'),
    'adoctl-v0.1.0-aarch64-apple-darwin.tar.gz',
  );
  assert.equal(
    artifactName('x86_64-pc-windows-msvc', '0.1.0'),
    'adoctl-v0.1.0-x86_64-pc-windows-msvc.zip',
  );
  assert.equal(
    releaseBaseUrl('0.1.0'),
    'https://github.com/doggy8088/adoctl/releases/download/v0.1.0',
  );
  assert.equal(binaryName('x86_64-pc-windows-msvc'), 'adoctl.exe');
});

test('解析並驗證 SHA256SUMS', () => {
  const directory = mkdtempSync(join(tmpdir(), 'adoctl-npm-test-'));
  const file = join(directory, 'sample.txt');
  writeFileSync(file, 'hello');
  const digest = sha256(file);
  const checksums = parseChecksums(`${digest}  sample.txt\n`);
  assert.equal(checksums.get('sample.txt'), digest);
  verifyChecksum(file, `${digest}  sample.txt\n`, 'sample.txt');
  assert.throws(
    () => verifyChecksum(file, `${'0'.repeat(64)}  sample.txt\n`, 'sample.txt'),
    /SHA-256 不一致/,
  );
  assert.throws(() => parseChecksums('not-a-checksum sample.txt'), /格式無效/);
});

test('發布前要求六個平台資產與單一 SHA256SUMS', () => {
  const urls = expectedReleaseUrls('0.1.0');
  assert.equal(urls.length, 7);
  assert.equal(
    urls.at(-1),
    'https://github.com/doggy8088/adoctl/releases/download/v0.1.0/SHA256SUMS',
  );
});

test('發布資產檢查支援成功及明確失敗', async () => {
  const success = await verifyReleaseAssets({
    version: '0.1.0',
    check: async (url) => ({ url, ok: true, statusCode: 200 }),
  });
  assert.equal(success.length, 7);

  await assert.rejects(
    verifyReleaseAssets({
      version: '0.1.0',
      check: async (url) => ({ url, ok: false, statusCode: 404 }),
    }),
    /缺少可下載的 npm 原生資產/,
  );
});
