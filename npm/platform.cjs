'use strict';

const { homedir } = require('node:os');
const { join, resolve } = require('node:path');

const TARGETS = Object.freeze({
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-arm64-gnu': 'aarch64-unknown-linux-gnu',
  'linux-x64-gnu': 'x86_64-unknown-linux-gnu',
  'linux-x64-musl': 'x86_64-unknown-linux-musl',
  'win32-x64': 'x86_64-pc-windows-msvc',
});

function detectLinuxLibc(
  override = process.env.ADOCTL_LIBC,
  reportHeader = process.report?.getReport()?.header,
) {
  if (override !== undefined && override !== '') {
    const normalized = override.toLowerCase();
    if (normalized !== 'gnu' && normalized !== 'musl') {
      throw new Error('ADOCTL_LIBC 只接受 gnu 或 musl。');
    }
    return normalized;
  }

  return reportHeader?.glibcVersionRuntime ? 'gnu' : 'musl';
}

function platformKey(platform = process.platform, arch = process.arch, libc) {
  if (platform === 'linux') {
    return `${platform}-${arch}-${libc ?? detectLinuxLibc()}`;
  }
  return `${platform}-${arch}`;
}

function cargoTarget(platform = process.platform, arch = process.arch, libc) {
  const key = platformKey(platform, arch, libc);
  const target = TARGETS[key];
  if (!target) {
    const libcSuffix = platform === 'linux' ? `，libc=${libc ?? detectLinuxLibc()}` : '';
    throw new Error(`adoctl npm 套件不支援目前平台：${platform}/${arch}${libcSuffix}。`);
  }
  return target;
}

function binaryName(target) {
  return target === 'x86_64-pc-windows-msvc' ? 'adoctl.exe' : 'adoctl';
}

function cacheRoot({
  env = process.env,
  platform = process.platform,
  home = homedir(),
} = {}) {
  if (env.ADOCTL_CACHE_DIR) {
    return resolve(env.ADOCTL_CACHE_DIR);
  }
  if (platform === 'darwin') {
    return join(home, 'Library', 'Caches', 'adoctl');
  }
  if (platform === 'win32') {
    return join(env.LOCALAPPDATA || join(home, 'AppData', 'Local'), 'adoctl');
  }
  return join(env.XDG_CACHE_HOME || join(home, '.cache'), 'adoctl');
}

function cachedBinaryPath(version, target, options) {
  return join(cacheRoot(options), `v${version}`, target, binaryName(target));
}

module.exports = {
  TARGETS,
  binaryName,
  cacheRoot,
  cachedBinaryPath,
  cargoTarget,
  detectLinuxLibc,
  platformKey,
};
