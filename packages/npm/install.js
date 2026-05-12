const fs = require('fs');
const path = require('path');
const https = require('https');
const { pipeline } = require('stream');
const { promisify } = require('util');
const { execSync } = require('child_process');

const pipe = promisify(pipeline);

const PLATFORM_MAP = {
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
};

const BINARY_NAME = process.platform === 'win32' ? 'devguard.exe' : 'devguard';

const VERSION = require('./package.json').version;
const BASE_URL = process.env.DEVGUARD_BINARY_URL
  || `https://github.com/anthonystepvoy/devguard/releases/download/v${VERSION}`;

async function download(url, destPath) {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        https.get(res.headers.location, (redirectRes) => {
          const file = fs.createWriteStream(destPath);
          pipe(redirectRes, file).then(resolve, reject);
        }).on('error', reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        return;
      }
      const file = fs.createWriteStream(destPath);
      pipe(res, file).then(resolve, reject);
    }).on('error', reject);
  });
}

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

  const isWindows = process.platform === 'win32';
  const ext = isWindows ? '.exe' : '';
  const rawUrl = `${BASE_URL}/devguard-${target}${ext}`;

  console.log(`devguard: downloading ${rawUrl}`);

  try {
    await download(rawUrl, binPath);
  } catch {
    const archiveExt = isWindows ? '.zip' : '.tar.gz';
    const archiveUrl = `${BASE_URL}/devguard-${target}${archiveExt}`;
    const archivePath = binPath + archiveExt;

    console.log(`devguard: downloading ${archiveUrl}`);
    await download(archiveUrl, archivePath);

    if (isWindows) {
      execSync(`powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${binDir}' -Force"`, { stdio: 'ignore' });
      fs.unlinkSync(archivePath);
      const extracted = path.join(binDir, BINARY_NAME);
      if (fs.existsSync(extracted) && extracted !== binPath) {
        fs.renameSync(extracted, binPath);
      }
    } else {
      execSync(`tar -xzf "${archivePath}" -C "${binDir}"`, { stdio: 'ignore' });
      fs.unlinkSync(archivePath);
    }
  }

  if (process.platform !== 'win32') {
    fs.chmodSync(binPath, 0o755);
  }

  console.log(`devguard: installed ${binPath}`);
}

main().catch((err) => {
  console.error('devguard: failed to install binary');
  console.error(err.message);
  process.exit(1);
});
