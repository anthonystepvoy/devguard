const assert = require('node:assert/strict');
const test = require('node:test');

const { assertSafeDownloadUrl, parseSha256 } = require('./install');

test('parseSha256 accepts common checksum file formats', () => {
  const hash = 'a'.repeat(64);
  assert.equal(parseSha256(`${hash}\n`), hash);
  assert.equal(parseSha256(`${hash}  devguard.exe\n`), hash);
});

test('parseSha256 rejects malformed checksums', () => {
  assert.throws(() => parseSha256('not-a-checksum'), /invalid sha256 file/);
});

test('assertSafeDownloadUrl rejects non-HTTPS URLs by default', () => {
  assert.doesNotThrow(() => assertSafeDownloadUrl('https://example.com/devguard'));
  assert.throws(
    () => assertSafeDownloadUrl('http://example.com/devguard'),
    /refusing non-HTTPS binary URL/,
  );
});
