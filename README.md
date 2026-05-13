# devguard

Experimental HOME/env isolation for package-manager install scripts.

```bash
npm install -g @anthonystepvoy/devguard
devguard install npm
```

devguard reduces the secrets exposed to dependency lifecycle scripts by running scripts with a temporary `HOME`, redirected Windows home variables, no SSH agent socket, and a conservative environment allowlist.

It is not an OS sandbox yet. Scripts can still read files through absolute paths, can read files in the project directory, and can use the network. Treat this as a defense-in-depth install wrapper, not a complete token firewall.

## The Problem

Developer machines often keep high-value bearer tokens in files that package install scripts can read:

| Token location | What it can grant |
|---|---|
| `~/.npmrc` | npm registry auth |
| `~/.ssh` | repository and server access |
| `~/.aws/credentials` | cloud infrastructure access |
| `~/.config/gh/hosts.yml` | GitHub CLI auth |
| `~/.docker/config.json` | registry auth |
| `~/.kube/config` | cluster access |
| shell history | previously pasted credentials |

Scanners and package reputation tools help, but a novel malicious lifecycle script still runs as your user. devguard's current approach is to remove common home-directory and environment-secret access paths during the script phase.

## How It Works

### npm / pnpm

```text
Phase 1: npm install --ignore-scripts   # real HOME, real auth, scripts disabled
Phase 2: npm rebuild                    # temporary HOME, scripts enabled
```

Private registries can still work during download. Lifecycle scripts then run with `HOME`, `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, `APPDATA`, and `LOCALAPPDATA` pointed at a temporary directory.

### yarn / bun

yarn and bun do not have an equivalent rebuild flow here, so devguard uses a single-pass temporary HOME and copies only package-manager auth/config files into it. This is weaker because auth files can be visible to lifecycle scripts.

## Commands

```bash
devguard scan
devguard scan --json
devguard scan --dir ./project

devguard install npm
devguard install pnpm
devguard install yarn
devguard install bun
devguard install npm --cwd ./my-project
devguard install npm -- -D typescript

devguard audit-log
devguard audit-log --lines 50

devguard doctor
```

## What Is Protected Today

Scripts that use `$HOME`, `~`, `USERPROFILE`, `HOMEDRIVE`/`HOMEPATH`, `APPDATA`, or `LOCALAPPDATA` see a temporary directory instead of your real home.

devguard also removes SSH agent variables and passes only a small environment allowlist, which keeps common variables such as `GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`, `NPM_TOKEN`, and generic `*_TOKEN`, `*_SECRET`, `*_PASSWORD`, `*_API_KEY`, `*_PRIVATE_KEY`, and `*_CREDENTIAL` names out of the script environment.

## Current Limitations

- This is HOME/env redirection, not OS-level filesystem isolation.
- Absolute paths such as `C:\Users\name\.npmrc` or `/home/name/.ssh/id_rsa` can bypass the protection.
- Files in the project directory, including project `.env` files, remain visible to dependency scripts.
- Network policy is advisory only; devguard does not block exfiltration yet.
- yarn/bun isolation is weaker than npm/pnpm because auth files may be present in the temporary HOME.
- macOS and Linux need real-world validation.

OS-level enforcement with AppContainer, Landlock, and macOS security APIs is the main v0.2 direction.

## Install

### npm

```bash
npm install -g @anthonystepvoy/devguard
```

The npm package downloads a platform binary from GitHub releases and verifies a matching `.sha256` file. Release assets must use these names:

```text
devguard-x86_64-pc-windows-msvc.exe
devguard-aarch64-pc-windows-msvc.exe
devguard-x86_64-apple-darwin
devguard-aarch64-apple-darwin
devguard-x86_64-unknown-linux-gnu
devguard-aarch64-unknown-linux-gnu
```

Each asset needs a sidecar checksum file named `<asset>.sha256`.

### Build From Source

```bash
git clone https://github.com/anthonystepvoy/devguard
cd devguard
cargo build --release
```

The binary is `target/release/devguard` or `target/release/devguard.exe`.

## Roadmap

- [x] Token scanner with redacted output
- [x] npm/pnpm two-phase HOME/env isolation
- [x] yarn/bun temporary HOME fallback
- [x] Conservative environment allowlist
- [x] JSONL audit log
- [x] npm distribution wrapper with checksum verification
- [x] `devguard doctor` diagnostics
- [x] CI malicious lifecycle-script fixture
- [ ] OS-level filesystem isolation
- [ ] OS-level network blocking
- [ ] Per-project policies
- [ ] Project `.env` mediation
- [ ] Token broker daemon
- [ ] macOS and Linux validation

## License

MIT
