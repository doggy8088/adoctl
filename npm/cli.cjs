#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const { join } = require('node:path');
const { cachedBinaryPath, cargoTarget } = require('./platform.cjs');

const version = require('../package.json').version;

let target;
try {
  target = cargoTarget();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const binary = cachedBinaryPath(version, target);
if (!existsSync(binary)) {
  const downloader = spawnSync(process.execPath, [join(__dirname, 'download.cjs')], {
    stdio: 'inherit',
  });
  if (downloader.error) {
    console.error(`無法啟動 adoctl 下載程式：${downloader.error.message}`);
    process.exit(1);
  }
  if (downloader.status !== 0 || !existsSync(binary)) {
    process.exit(downloader.status ?? 1);
  }
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  console.error(`無法啟動 adoctl：${result.error.message}`);
  process.exit(1);
}

if (result.signal) {
  console.error(`adoctl 因訊號 ${result.signal} 結束。`);
  process.exit(1);
}

process.exit(result.status ?? 1);
