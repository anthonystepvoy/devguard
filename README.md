# devguard

**Developer token firewall — run package installs without exposing your secrets.**

```
npm install -g devguard    # or pnpm, yarn, bun
devguard install npm       # no access to ~/.ssh, ~/.aws, ~/.npmrc, ~/.config/gh, .env
```

## The problem

Your machine holds valuable tokens in files any `npm install` script can read:

| Token location | What it grants |
|---|---|
| `~/.npmrc` | Publish packages under your name |
| `~/.ssh` | Push to your repos |
| `~/.aws/credentials` | Provision infrastructure |
| `~/.config/gh/hosts.yml` | Full GitHub account access |
| `.env` | Database URLs, API keys |
| Shell history | Leaked credentials in past commands |

Recent supply-chain attacks (Mini Shai-Hulud, CanisterWorm, Intercom compromise — all 2026) abused postinstall scripts to read exactly these files. Scanners help, but zero-day malware slips through. The only reliable defense is **denying access at runtime**.

## How it works

devguard wraps your package manager in a sandboxed environment during install:

```
devguard install npm
```

### npm / pnpm — two-phase isolation

```
Phase 1: npm install --ignore-scripts   ← real HOME, real .npmrc (download only)
Phase 2: npm rebuild                    ← fake HOME, no secrets (scripts run here)
```

Scripts execute with access to nothing. Private registries work because auth is only used during download.

### yarn / bun — union HOME

A temporary HOME is created with only the auth files needed for registry access. Everything else is blocked.

## Commands

```bash
# Scan your machine for exposed tokens
devguard scan
devguard scan --json
devguard scan --dir ./project

# Run a sandboxed package install
devguard install npm
devguard install pnpm
devguard install yarn
devguard install bun
devguard install npm -- -D typescript          # extra args
devguard install npm --cwd ./my-project

# View audit log
devguard audit-log
devguard audit-log --lines 50
```

## What gets hidden

Scripts that access files through `$HOME` or `~` see an empty temp directory:

```
✓ ~/.ssh          → hidden via HOME redirect
✓ ~/.aws          → hidden via HOME redirect
✓ ~/.config/gh    → hidden via HOME redirect
✓ ~/.docker       → hidden via HOME redirect
✓ ~/.kube         → hidden via HOME redirect
✓ ~/.git-credentials → hidden via HOME redirect
✓ ~/.netrc        → hidden via HOME redirect
✓ ~/.azure, ~/.config/gcloud → hidden via HOME redirect
✓ SSH agent socket → disconnected
✓ Secret env vars → stripped (GITHUB_TOKEN, AWS_*, etc.)

npm/pnpm only:
✓ ~/.npmrc        → fully blocked (not present in sandbox)
```

> **Current limitation:** Scripts using absolute paths (e.g. `/home/user/.ssh` or `C:\Users\Admin\.ssh`) can still access these files. The current defense is HOME redirection, not OS-level filesystem isolation. Full sandbox enforcement via Landlock, AppContainer, and Endpoint Security is planned for v0.2.

## Install

### npm / pnpm / yarn / bun

```bash
npm install -g devguard
pnpm add -g devguard
yarn global add devguard
bun add -g devguard
```

### Build from source

```bash
git clone https://github.com/devguard/devguard
cd devguard
cargo build --release
# binary at target/release/devguard
```

## Roadmap

- [x] Token scanner (22 file paths, 16 regex patterns)
- [x] Sandboxed install with fake HOME
- [x] Two-phase isolation (npm/pnpm)
- [x] Union HOME fallback (yarn/bun)
- [x] Secret env var stripping
- [x] Audit logging (JSONL)
- [x] npm distribution wrapper
- [ ] OS-level network blocking (WFP, nftables/eBPF, Network Extension)
- [ ] OS-level filesystem blocking (AppContainer, Landlock, ESF)
- [ ] macOS support validation
- [ ] Per-project trust policies
- [ ] Token broker daemon (hardware-backed secret storage)
- [ ] Transparent background protection

## FAQ

**Does this slow down installs?**  
Minimal. For npm/pnpm, the two-phase approach adds one `npm rebuild` pass after the normal download — typically under 1 second. For yarn/bun, it's a single pass.

**Does it break native packages like esbuild or sharp?**  
No. Build tools (node-gyp, ccache, etc.) work through standard cache directories that are created in the sandbox HOME.

**What about private registries?**  
Fully supported. Auth files pass through during the download phase. Scripts run without access to them.

**Which OS?**  
Windows — built and tested. The code is cross-platform Rust; Linux and macOS should compile but haven't been verified. If you can test on those platforms, contributions are very welcome (see [Contributing](#contributing)).

## License

MIT

## Contributing

Contributions are welcome. Big areas where help is especially needed:

- **macOS testing and validation** — I don't have access to a Mac. The Rust code should be portable but needs real-world testing and possible sandbox-exec/Endpoint Security integration.
- **Linux testing and validation** — should work out of the box but hasn't been tested. Linux also opens the door to namespace/seccomp/Landlock enforcement.
- **OS-level network blocking** — Windows Filtering Platform, Linux nftables/eBPF, macOS Network Extension.
- **More package ecosystems** — pip, cargo, gem, nuget.
- **Better Windows sandboxing** — AppContainer integration for true filesystem isolation.
- **Tests** — there are none yet.

Open an issue or PR if you want to help. All skill levels welcome.
