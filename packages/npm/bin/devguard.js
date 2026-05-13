#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const BINARY_NAME = process.platform === 'win32' ? 'devguard.exe' : 'devguard';
const binPath = path.join(__dirname, BINARY_NAME);

if (!fs.existsSync(binPath)) {
  console.error('devguard: binary not found. Run `npm install @anthonystepvoy/devguard` to download it.');
  process.exit(1);
}

const args = process.argv.slice(2);

const result = spawnSync(binPath, args, {
  stdio: 'inherit',
  env: process.env,
  windowsHide: true,
});

process.exit(result.status ?? 1);
