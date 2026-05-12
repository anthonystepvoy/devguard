#!/usr/bin/env node

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const BINARY_NAME = process.platform === 'win32' ? 'devguard.exe' : 'devguard';
const binPath = path.join(__dirname, BINARY_NAME);

if (!fs.existsSync(binPath)) {
  console.error('devguard: binary not found. Run `npm install devguard` to download it.');
  process.exit(1);
}

const args = process.argv.slice(2);

try {
  execSync(`"${binPath}" ${args.map(a => `"${a}"`).join(' ')}`, {
    stdio: 'inherit',
    env: process.env,
  });
} catch (err) {
  process.exit(err.status || 1);
}
