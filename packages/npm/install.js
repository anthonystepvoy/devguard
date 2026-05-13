const fs = require('fs');
const path = require('path');
const https = require('https');
const crypto = require('crypto');
const { pipeline } = require('stream');
const { promisify } = require('util');
const { execFileSync } = require('child_process');

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

function assertSafeDownloadUrl(url) {
  const parsed = new URL(url);
  if (parsed.protocol === 'https:') {
    return;
  }
  if (process.env.DEVGUARD_ALLOW_INSECURE_BINARY_URL === '1') {
    return;
  }
  throw new Error(`refusing non-HTTPS binary URL: ${url}`);
}

function parseSha256(text) {
  const match = text.trim().match(/^([a-fA-F0-9]{64})(?:\s+.*)?$/);
  if (!match) {
    throw new Error('invalid sha256 file');
  }
  return match[1].toLowerCase();
}

function sha256File(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

async function download(url, destPath, redirects = 0) {
  assertSafeDownloadUrl(url);

  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        if (redirects >= 5) {
          reject(new Error(`too many redirects: ${url}`));
          return;
        }
        const nextUrl = new URL(res.headers.location, url).toString();
        res.resume();
        download(nextUrl, destPath, redirects + 1).then(resolve, reject);
        return;
      }

      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        return;
      }

      const file = fs.createWriteStream(destPath);
      pipe(res, file).then(resolve, reject);
    }).on('error', reject);
  });
}

async function downloadText(url, redirects = 0) {
  assertSafeDownloadUrl(url);

  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        if (redirects >= 5) {
          reject(new Error(`too many redirects: ${url}`));
          return;
        }
        const nextUrl = new URL(res.headers.location, url).toString();
        res.resume();
        downloadText(nextUrl, redirects + 1).then(resolve, reject);
        return;
      }

      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        return;
      }

      res.setEncoding('utf8');
      let body = '';
      res.on('data', (chunk) => {
        body += chunk;
      });
      res.on('end', () => resolve(body));
    }).on('error', reject);
  });
}

async function verifySha256(assetUrl, filePath) {
  if (process.env.DEVGUARD_SKIP_CHECKSUM === '1') {
    console.warn('devguard: checksum verification skipped by DEVGUARD_SKIP_CHECKSUM=1');
    return;
  }

  const expected = parseSha256(await downloadText(`${assetUrl}.sha256`));
  const actual = sha256File(filePath);
  if (actual !== expected) {
    throw new Error(`checksum mismatch for ${path.basename(filePath)}`);
  }
}

async function downloadVerified(assetUrl, destPath) {
  await download(assetUrl, destPath);
  await verifySha256(assetUrl, destPath);
}

function extractArchive(archivePath, binDir, isWindows) {
  if (isWindows) {
    execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        'Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force',
        archivePath,
        binDir,
      ],
      { stdio: 'ignore' },
    );
    return;
  }

  execFileSync('tar', ['-xzf', archivePath, '-C', binDir], { stdio: 'ignore' });
}

function findBinary(rootDir) {
  const queue = [rootDir];
  while (queue.length > 0) {
    const current = queue.shift();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(entryPath);
      } else if (entry.name === BINARY_NAME) {
        return entryPath;
      }
    }
  }
  return null;
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

  fs.mkdirSync(binDir, { recursive: true });

  const isWindows = process.platform === 'win32';
  const ext = isWindows ? '.exe' : '';
  const rawUrl = `${BASE_URL}/devguard-${target}${ext}`;

  console.log(`devguard: downloading ${rawUrl}`);

  try {
    await downloadVerified(rawUrl, binPath);
  } catch (rawError) {
    const archiveExt = isWindows ? '.zip' : '.tar.gz';
    const archiveUrl = `${BASE_URL}/devguard-${target}${archiveExt}`;
    const archivePath = binPath + archiveExt;

    console.log(`devguard: direct binary unavailable (${rawError.message})`);
    console.log(`devguard: downloading ${archiveUrl}`);
    await downloadVerified(archiveUrl, archivePath);
    extractArchive(archivePath, binDir, isWindows);
    fs.unlinkSync(archivePath);

    if (!fs.existsSync(binPath)) {
      const extracted = findBinary(binDir);
      if (extracted && extracted !== binPath) {
        fs.renameSync(extracted, binPath);
      }
    }
  }

  if (!fs.existsSync(binPath)) {
    throw new Error(`binary not found after install: ${binPath}`);
  }

  if (process.platform !== 'win32') {
    fs.chmodSync(binPath, 0o755);
  }

  console.log(`devguard: installed ${binPath}`);
}

if (require.main === module) {
  main().catch((err) => {
    console.error('devguard: failed to install binary');
    console.error(err.message);
    process.exit(1);
  });
}

module.exports = {
  assertSafeDownloadUrl,
  parseSha256,
  sha256File,
};
