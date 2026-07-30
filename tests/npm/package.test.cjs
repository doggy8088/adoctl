'use strict';

const assert = require('node:assert/strict');
const { mkdtempSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const test = require('node:test');

const {
  archiveName,
  parseChecksums,
  sha256,
  validateArchiveEntries,
} = require('../../npm/prepare-package.cjs');
const {
  binaryName,
  cargoTarget,
  detectLinuxLibc,
  platformKey,
} = require('../../npm/platform.cjs');

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

test('產生與 GitHub Release 一致的封裝名稱', () => {
  assert.equal(
    archiveName('0.1.0', 'aarch64-apple-darwin'),
    'adoctl-v0.1.0-aarch64-apple-darwin.tar.gz',
  );
  assert.equal(
    archiveName('0.1.0', 'x86_64-pc-windows-msvc'),
    'adoctl-v0.1.0-x86_64-pc-windows-msvc.zip',
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
  assert.throws(() => parseChecksums('not-a-checksum sample.txt'), /格式無效/);
});

test('只接受位於封裝根目錄的三個預期檔案', () => {
  validateArchiveEntries(['adoctl', 'README.md', 'CHANGELOG.md'], 'x86_64-apple-darwin');
  validateArchiveEntries(
    ['adoctl.exe', 'CHANGELOG.md', 'README.md'],
    'x86_64-pc-windows-msvc',
  );
  assert.throws(
    () =>
      validateArchiveEntries(
        ['folder/adoctl', 'README.md', 'CHANGELOG.md'],
        'x86_64-apple-darwin',
      ),
    /封裝內容不符預期/,
  );
});
