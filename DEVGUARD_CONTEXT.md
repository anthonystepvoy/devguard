# devguard Context Summary

Last updated: 2026-05-13

This file is the quick-start context for future chats and agents working on this repo.

## What devguard Is

devguard is an experimental package-manager install wrapper that reduces what dependency lifecycle scripts can see during installs.

It is aimed at npm supply-chain attacks where a malicious `postinstall` script reads developer-machine secrets such as:

- `~/.npmrc`
- `~/.ssh`
- `~/.aws/credentials`
- `~/.config/gh/hosts.yml`
- `~/.docker/config.json`
- `~/.kube/config`
- `~/.env`
- secret environment variables such as `GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`, `OPENAI_API_KEY`, `NPM_TOKEN`

The current version is a defense-in-depth HOME/env isolation tool. It is not yet a full OS sandbox or network firewall.

## Current Release Status

- GitHub repo: `https://github.com/anthonystepvoy/devguard`
- npm package: `@anthonystepvoy/devguard`
- Cargo package name: `devguard-token-firewall`
- CLI binary name: `devguard`
- Latest release at time of writing: `0.1.1`
- GitHub releases contain platform binaries plus `.sha256` checksum files.
- npm installer downloads the matching GitHub release binary and verifies the sidecar checksum.

Install:

```powershell
npm install -g @anthonystepvoy/devguard --ignore-scripts=false
```

Basic commands:

```powershell
devguard --version
devguard scan
devguard doctor
devguard install npm
devguard audit-log
```

## What Works in v0.1.x

### npm / pnpm Two-Phase Isolation

For npm and pnpm, devguard uses two phases:

```text
Phase 1: npm install --ignore-scripts
  - real HOME
  - real .npmrc
  - scripts disabled
  - package downloads work, including private registries

Phase 2: npm rebuild
  - fake temporary HOME
  - stripped environment
  - scripts enabled
```

This means lifecycle scripts using `HOME`, `~`, `USERPROFILE`, `APPDATA`, or similar home-based paths see a temporary directory instead of the real user home.

### yarn / bun Fallback

yarn and bun do not have the same rebuild flow here, so devguard uses a temporary HOME and copies package-manager auth/config files into it.

This mode is weaker because auth files may be visible to lifecycle scripts.

### Environment Stripping

devguard now uses a conservative allowlist for environment variables and removes common secret names:

- `*_TOKEN`
- `*_SECRET`
- `*_PASSWORD`
- `*_API_KEY`
- `*_PRIVATE_KEY`
- `*_CREDENTIAL`
- `NPM_TOKEN`
- `NODE_AUTH_TOKEN`
- `GITHUB_TOKEN`
- `AWS_SECRET_ACCESS_KEY`
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_API_KEY`

The malicious fixture test confirms these fake env vars are not visible to a postinstall script:

- `OPENAI_API_KEY`
- `GITHUB_TOKEN`
- `AWS_SECRET_ACCESS_KEY`
- `NPM_TOKEN`

### Scanner

`devguard scan` checks known secret file paths and token patterns.

Important fixes already made:

- `scan --dir` now scans known secret paths under the requested directory, not the real home.
- JSON token snippets are redacted.

### Doctor

`devguard doctor` checks:

- version
- OS and architecture
- home directory
- temp directory writability
- audit log directory
- package-manager discovery and versions
- npm `ignore-scripts` setting
- count of known home secret paths
- known limitations

`devguard --no-color doctor` is useful on shells that render ANSI escapes poorly.

## Current Limitations

Be precise about these in docs, posts, and diagrams:

- v0.1.x is HOME/env isolation, not OS-level filesystem sandboxing.
- Absolute paths to real home files can bypass protection.
  - Example: `C:\Users\name\.npmrc`
  - Example: `/home/name/.ssh/id_rsa`
- Project files remain visible to lifecycle scripts.
  - This includes project `.env` files.
- Network blocking is advisory only.
- yarn/bun isolation is weaker than npm/pnpm.
- macOS/Linux have CI build coverage, but real-world usage still needs testing.

Do not claim:

- "full firewall"
- "network blocked"
- "project `.env` hidden"
- "absolute paths blocked"
- "complete OS sandbox"

Safe phrasing:

```text
Experimental HOME/env isolation for package-manager lifecycle scripts. Not a full OS sandbox yet.
```

## Repo Structure

```text
src/main.rs       CLI dispatch
src/scanner.rs    token/secret file scanner
src/sandbox.rs    install orchestration and HOME/env isolation
src/env.rs        environment allowlist and secret variable stripping
src/paths.rs      known secret paths, sandbox dirs, env secret patterns
src/network.rs    advisory network policy text
src/audit.rs      JSONL audit logging
src/doctor.rs     setup diagnostics
src/console.rs    Windows VT color setup and --no-color support

packages/npm/
  package.json    npm wrapper package metadata
  install.js      platform binary downloader with sha256 verification
  bin/devguard.js JS shim that executes downloaded native binary
  README.md       npm package README

.github/workflows/ci.yml       Windows + Ubuntu CI, includes malicious lifecycle fixture
.github/workflows/release.yml  tag-based multi-platform release build

docs/windows-appcontainer-v0.2.md  Windows OS sandboxing plan
```

## CI and Release

CI runs on:

- `windows-latest`
- `ubuntu-latest`

CI checks:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --locked -- -D warnings`
- npm wrapper tests
- CLI build
- malicious lifecycle fixture

Release workflow triggers on `v*` tags and builds:

- `devguard-x86_64-pc-windows-msvc.exe`
- `devguard-aarch64-pc-windows-msvc.exe`
- `devguard-x86_64-apple-darwin`
- `devguard-aarch64-apple-darwin`
- `devguard-x86_64-unknown-linux-gnu`
- `devguard-aarch64-unknown-linux-gnu`

Each asset gets a matching `.sha256` file.

Release flow:

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -D warnings
npm test --prefix packages/npm

git tag -a vX.Y.Z -m "devguard vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

Wait for GitHub release workflow to finish, then:

```powershell
cd packages/npm
npm publish --access public --auth-type=web
```

The npm account uses passkey/web auth, so automated publish from this environment generally cannot complete without user interaction.

## Known Local Environment Notes

On this Windows machine:

- npm config currently has `ignore-scripts=true`.
- Installing devguard from npm for smoke tests needs `--ignore-scripts=false`.
- PowerShell may render ANSI escapes oddly in some contexts. Use `--no-color`.

Example:

```powershell
npm install -g @anthonystepvoy/devguard --ignore-scripts=false
devguard --no-color doctor
```

## Smoke Test Fixture

The malicious fixture creates a local package with a `postinstall` script that writes a report containing visible env vars and HOME file existence checks.

Expected result under `devguard install npm`:

```json
{
  "seenEnv": {
    "OPENAI_API_KEY": null,
    "GITHUB_TOKEN": null,
    "AWS_SECRET_ACCESS_KEY": null,
    "NPM_TOKEN": null
  },
  "homeNpmrcExists": false,
  "homeSshExists": false
}
```

This proves the intended v0.1 model: scripts using HOME/env do not see those secrets.

## Twitter / Public Positioning

Safe post framing:

```text
shipped devguard v0.1 alpha

it wraps npm/pnpm/yarn/bun installs so lifecycle scripts run with:
- fake HOME / USERPROFILE
- no ~/.npmrc / ~/.ssh via normal home lookups
- secret env vars stripped
- audit log + token scanner

not OS sandboxing yet. absolute paths and project .env are still limitations.

npm install -g @anthonystepvoy/devguard
devguard install npm

looking for people to try it on real projects and break it:
https://github.com/anthonystepvoy/devguard
```

Diagram accuracy notes:

- Use `~/.env`, not generic `.env`, when showing protected files.
- Show project files remain visible if the diagram mentions `.env`.
- Red arrows on protected side should stop at shields/X marks, not reach locked files.
- Mention "network not blocked, but no HOME/env secrets to send".

## v0.2 Direction

Primary next technical goal: Windows AppContainer backend.

See:

```text
docs/windows-appcontainer-v0.2.md
```

High-level plan:

- keep existing fake HOME/env stripping
- create/reuse AppContainer profile
- launch lifecycle phase with AppContainer token
- grant AppContainer SID access only to project dir and sandbox HOME
- grant no network capability by default
- test absolute-path denial and network denial

Acceptance tests for v0.2:

- absolute path read of real `.npmrc` fails
- absolute path read of real `.ssh` fails
- fake env vars remain stripped
- outbound network fails by default
- native package rebuilds still work
- `--allow-network` explicitly permits network when needed

After Windows:

- Linux Landlock backend
- macOS sandbox/Endpoint Security exploration
- per-project policy
- project secret mediation for `.env`

## Important Lessons From v0.1 Audit

Original claims were too strong. The implementation was useful but not a true firewall.

Issues fixed before public release:

- npm package name conflict: now scoped as `@anthonystepvoy/devguard`
- crates.io name conflict: Cargo package renamed to `devguard-token-firewall`
- npm installer now checks `.sha256`
- README and CLI now avoid full-sandbox claims
- scanner `--dir` correctness fixed
- scanner JSON redaction added
- env stripping broadened
- CI added
- release assets automated

Remaining honest limitation:

```text
devguard v0.1.x stops common HOME/env token grabs, not all local file access.
```
