'use strict';

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

module.exports = {
  TARGETS,
  binaryName,
  cargoTarget,
  detectLinuxLibc,
  platformKey,
};
