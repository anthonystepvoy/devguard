# Windows AppContainer v0.2 Plan

This is the recommended path for turning devguard's Windows install isolation from HOME/env redirection into OS-enforced filesystem and network denial.

## Why AppContainer

Microsoft describes AppContainer as a sandboxed execution environment that restricts access to system resources, other apps, user data, credentials, files, registry keys, network, processes, and windows unless access is explicitly granted. AppContainer access is controlled through package SIDs, capability SIDs, process tokens, integrity level, and DACLs.

For devguard, that maps directly to the v0.2 goal:

- block absolute-path reads of real home secrets such as `C:\Users\name\.npmrc` and `C:\Users\name\.ssh`
- block outbound network by default during lifecycle scripts
- grant only the project/package-manager paths needed for `npm rebuild`
- preserve the existing fake-HOME/env stripping layer as defense in depth

## Relevant Windows APIs

- `CreateAppContainerProfile`: creates a per-user, per-app AppContainer profile and returns the AppContainer SID.
- `DeriveAppContainerSidFromAppContainerName`: retrieves the SID for an existing profile.
- `GetAppContainerFolderPath`: retrieves the local app-data folder for the AppContainer profile.
- `DeriveCapabilitySidsFromName`: creates capability SIDs such as `internetClient`.
- `STARTUPINFOEX` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`: tells `CreateProcess` to launch the child inside the AppContainer.
- `PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY`: needed for Less-Privileged AppContainer (LPAC), which is stricter than regular AppContainer.

## Proposed v0.2 Shape

Add a Windows-only sandbox backend:

```text
src/windows_appcontainer.rs
  create_or_open_profile()
  build_capabilities()
  grant_project_acl()
  build_container_env()
  spawn_appcontainer_process()
```

Keep `src/sandbox.rs` as the cross-platform orchestration layer:

```text
npm/pnpm phase 1:
  npm install --ignore-scripts

npm/pnpm phase 2 on Windows:
  AppContainer + fake HOME + stripped env + no network capability by default
  npm rebuild

yarn/bun on Windows:
  AppContainer + union HOME fallback
```

## Profile Strategy

Use one reusable per-user profile:

```text
devguard.anthonystepvoy.v2
```

Constraints from `CreateAppContainerProfile`:

- name max: 64 characters
- allowed pattern: `[-_. A-Za-z0-9]+`
- profile is created for the current user
- if it already exists, derive the SID instead of failing

The AppContainer profile folder becomes the real OS-accessible `LOCALAPPDATA`, `TEMP`, and `TMP` for lifecycle scripts.

## Capability Strategy

Default mode should pass zero capabilities:

```text
capabilities = []
```

That should deny network by default. `devguard install npm --allow-network` can opt into:

```text
internetClient
privateNetworkClientServer, if explicitly requested later
```

Do not grant broad capabilities for v0.2. Start with no network and add capabilities only when tests prove a package-manager scenario requires them.

## ACL Strategy

AppContainer can only access resources granted through both the user token and the AppContainer/package SID. The parent devguard process must grant the AppContainer SID access to the minimum paths needed for lifecycle scripts:

- project directory, initially read/write/execute because native build scripts modify `node_modules`
- sandbox HOME/profile directory, read/write/execute
- package-manager cache dirs inside sandbox HOME
- maybe Node/npm executable paths, read/execute only, if default system-file access is insufficient

Do not grant access to:

- real user home
- `.npmrc`, `.ssh`, `.aws`, `.config/gh`, `.docker`, `.kube`
- parent shell history
- global credential stores

First prototype may use `icacls` to validate the model quickly. Production code should use Windows ACL APIs directly rather than shelling out.

## Spawn Strategy

Rust's normal `Command` API cannot attach AppContainer security capabilities. The Windows backend needs direct process creation:

1. Build `SECURITY_CAPABILITIES` with the AppContainer SID and selected capability SIDs.
2. Allocate and initialize `PROC_THREAD_ATTRIBUTE_LIST`.
3. Add `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`.
4. Call `CreateProcessW` with `EXTENDED_STARTUPINFO_PRESENT`.
5. Pass the same sanitized env devguard already builds.
6. Set current directory to the project.

For `npm.cmd`, test both:

- launching `cmd.exe /c npm.cmd rebuild`
- resolving npm's JS entry point and launching `node.exe <npm-cli.js> rebuild`

Prefer the direct `node.exe` path if `cmd.exe` behavior is flaky inside AppContainer.

## LPAC Decision

Start with regular AppContainer, then evaluate LPAC.

LPAC is stronger, but Microsoft documents that it requires additional explicit capabilities for resources regular AppContainers can already use, such as registry and COM. Node/npm may depend on enough Windows runtime behavior that starting with LPAC could slow the first working implementation.

Recommended sequence:

1. regular AppContainer, no network capability
2. prove lifecycle scripts still run for common packages
3. add automated absolute-path denial tests
4. evaluate LPAC behind `DEVGUARD_WINDOWS_LPAC=1`

## Acceptance Tests

Add Windows integration tests that run a malicious local package and assert:

- `fs.readFileSync(process.env.HOME + "\\.npmrc")` fails
- `fs.readFileSync("C:\\Users\\...\\.npmrc")` fails
- `fs.readFileSync("C:\\Users\\...\\.ssh\\id_rsa")` fails
- `process.env.GITHUB_TOKEN` is absent
- `fetch("https://example.com")` fails by default
- native lifecycle scripts can still write under `node_modules`
- `--allow-network` permits outbound network only when requested

## Risks

- ACL grants must be precise. Accidentally granting the real home directory to the package SID destroys the security boundary.
- Some package scripts may assume access to registry, COM, or broad user-profile paths.
- `npm.cmd` / `cmd.exe` launch behavior may need special handling.
- Project `.env` remains visible if the whole project tree is granted. Solving project-secret mediation is a separate policy problem.
- AppContainer is Windows-only; Linux still needs Landlock and macOS still needs a separate backend.

## Sources

- Microsoft Learn: AppContainer isolation  
  https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation
- Microsoft Learn: Launch an AppContainer  
  https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer
- Microsoft Learn: `CreateAppContainerProfile`  
  https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile
- Microsoft Learn: `DeriveAppContainerSidFromAppContainerName`  
  https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deriveappcontainersidfromappcontainername
- Microsoft Learn: `DeriveCapabilitySidsFromName`  
  https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-derivecapabilitysidsfromname
- Microsoft Learn: `GetAppContainerFolderPath`  
  https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-getappcontainerfolderpath
