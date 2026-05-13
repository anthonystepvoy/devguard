# @anthonystepvoy/devguard

Experimental HOME/env isolation for package-manager install scripts.

```bash
npm install -g @anthonystepvoy/devguard
devguard install npm
```

devguard is a defense-in-depth wrapper for npm, pnpm, yarn, and bun. It reduces what dependency lifecycle scripts can see by running the script phase with a temporary `HOME`, redirected Windows home variables, no SSH agent socket, and a conservative environment allowlist.

It is not an OS sandbox yet. Absolute paths to real home files, files inside the project directory, and outbound network access are still limitations.

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

devguard doctor
devguard audit-log
```

## How npm/pnpm Protection Works

```text
Phase 1: npm install --ignore-scripts   # real HOME, real auth, scripts disabled
Phase 2: npm rebuild                    # temporary HOME, scripts enabled
```

Private registry auth is available during download. Lifecycle scripts then run with `HOME`, `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, `APPDATA`, and `LOCALAPPDATA` pointed at a temporary directory.

## What Gets Hidden

- `~/.npmrc`, `~/.ssh`, `~/.aws`, `~/.config/gh`, `~/.docker`, `~/.kube` through normal HOME-based lookups
- SSH agent environment variables
- common secret environment variables such as `GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`, `NPM_TOKEN`
- generic `*_TOKEN`, `*_SECRET`, `*_PASSWORD`, `*_API_KEY`, `*_PRIVATE_KEY`, and `*_CREDENTIAL` names

## Current Limitations

- HOME/env redirection only; no OS-level filesystem sandbox yet
- absolute paths such as `/home/name/.ssh/id_rsa` or `C:\Users\name\.npmrc` can bypass v0.1.x protections
- project files, including project `.env`, remain visible to lifecycle scripts
- network policy is advisory only
- yarn/bun isolation is weaker because package-manager auth files may be copied into the temporary HOME

## Diagnostics

```bash
devguard doctor
```

Use `doctor` to check package-manager discovery, temp directory writability, npm `ignore-scripts` settings, and known secret-file exposure.

## Release Assets

The npm installer downloads a platform binary from GitHub releases and verifies a matching `.sha256` file before making it executable.

Repository: https://github.com/anthonystepvoy/devguard
