#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const { join } = require('node:path');
const { binaryName, cargoTarget } = require('./platform.cjs');

let target;
try {
  target = cargoTarget();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const binary = join(__dirname, 'native', target, binaryName(target));
if (!existsSync(binary)) {
  console.error(`找不到 ${target} 的 adoctl 原生執行檔；請重新安裝 npm 套件。`);
  process.exit(1);
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
