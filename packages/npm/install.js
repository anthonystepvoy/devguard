const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const https = require('https');
const { pipeline } = require('stream');
const { promisify } = require('util');

const pipe = promisify(pipeline);

const PLATFORM_MAP = {
  'win32-x64': 'x86_64-pc-windows-msvc',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
};

const BINARY_NAME = process.platform === 'win32' ? 'devguard.exe' : 'devguard';

const VERSION = require('./package.json').version;
const BASE_URL = process.env.DEVGUARD_BINARY_URL
  || `https://github.com/devguard/devguard/releases/download/v${VERSION}`;

async function main() {
  const platform = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[platform];

  if (!target) {
    console.error(`devguard: unsupported platform ${platform}`);
    console.error('Supported: win32-x64, darwin-x64, darwin-arm64, linux-x64, linux-arm64');
    process.exit(1);
  }

  const binDir = path.join(__dirname, 'bin');
  const binPath = path.join(binDir, BINARY_NAME);

  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const archiveName = process.platform === 'win32'
    ? `devguard-${target}.zip`
    : `devguard-${target}.tar.gz`;

  const url = `${BASE_URL}/${archiveName}`;

  console.log(`devguard: downloading ${url}`);

  const tmpPath = binPath + '.tmp';

  await new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        https.get(res.headers.location, resolve).on('error', reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        return;
      }
      const file = fs.createWriteStream(tmpPath);
      pipe(res, file).then(resolve, reject);
    }).on('error', reject);
  });

  if (process.platform !== 'win32') {
    fs.chmodSync(tmpPath, 0o755);
  }

  fs.renameSync(tmpPath, binPath);

  console.log(`devguard: installed ${binPath}`);
}

main().catch((err) => {
  console.error('devguard: failed to install binary');
  console.error(err.message);
  process.exit(1);
});
