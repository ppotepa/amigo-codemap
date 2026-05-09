# Amigo Codemap Daemon

`amigo-codemapd` is a local background process for Amigo Codemap.

Its goal is to avoid rebuilding the whole codemap snapshot for every CLI command.
The daemon keeps the current workspace codemap in memory, watches the repository for file changes, and serves fast local requests from `amigo-codemap`.

## Components

```text
amigo-codemap      CLI command
amigo-codemapd     local codemap daemon / workspace indexer
amigo-codemap-core shared logic inside the amigo-codemap crate
```

The daemon is a separate executable, but it uses the same Rust crate and scan logic as the normal CLI.

## Build

From the repository root:

```powershell
cargo build -p amigo-codemap --bins
```

Expected executables:

```text
target\debug\amigo-codemap.exe
target\debug\amigo-codemapd.exe
```

## Start daemon

From the repository root:

```powershell
.\target\debug\amigo-codemapd.exe run --root D:\Git\amigo --level 2
```

Default daemon address:

```text
127.0.0.1:47821
```

The daemon performs an initial scan, writes the codemap output, stores the snapshot cache, and then keeps running.

## Check daemon status

In another PowerShell window:

```powershell
.\target\debug\amigo-codemapd.exe status
```

Example output:

```text
codemap daemon status
  root: D:\Git\amigo
  out: D:\Git\amigo\.amigo\codemap.json
  level: 2
  ai: false
  dirty: false
  files: 1292
  packages: ...
  symbols: ...
  dependencies: ...
  areas: ...
```

`dirty: true` means the watcher noticed file changes and the next request may refresh the map.

## Use normal CLI

Once the daemon is running, use `amigo-codemap` normally:

```powershell
.\target\debug\amigo-codemap.exe brief
```

```powershell
.\target\debug\amigo-codemap.exe trace EditorTarget --limit 20
```

```powershell
.\target\debug\amigo-codemap.exe slice --file crates/apps/amigo-editor/src/App.tsx
```

```powershell
.\target\debug\amigo-codemap.exe anchors
```

The CLI tries to use the daemon automatically.

If the daemon is not running, the CLI falls back to the existing local snapshot/cache flow.

## Stop daemon

```powershell
.\target\debug\amigo-codemapd.exe shutdown
```

## Run daemon in background

From PowerShell:

```powershell
$daemon = Start-Process `
  -FilePath ".\target\debug\amigo-codemapd.exe" `
  -ArgumentList @("run", "--root", "D:\Git\amigo", "--level", "2") `
  -PassThru
```

Check status:

```powershell
.\target\debug\amigo-codemapd.exe status
```

Stop:

```powershell
.\target\debug\amigo-codemapd.exe shutdown
```

If needed, force stop:

```powershell
Stop-Process -Id $daemon.Id -Force
```

## Disable daemon usage

To force the old one-shot CLI behavior:

```powershell
$env:AMIGO_CODEMAP_NO_DAEMON = "1"
```

Then run:

```powershell
.\target\debug\amigo-codemap.exe brief
```

Remove the setting:

```powershell
Remove-Item Env:\AMIGO_CODEMAP_NO_DAEMON
```

## Show daemon fallback diagnostics

```powershell
$env:AMIGO_CODEMAP_DAEMON_VERBOSE = "1"
```

Then run any codemap command:

```powershell
.\target\debug\amigo-codemap.exe brief
```

If the daemon is unavailable, the CLI prints why it is falling back to local mode.

Remove the setting:

```powershell
Remove-Item Env:\AMIGO_CODEMAP_DAEMON_VERBOSE
```

## Change daemon address

Default:

```text
127.0.0.1:47821
```

Run daemon on a custom address:

```powershell
.\target\debug\amigo-codemapd.exe run --root D:\Git\amigo --level 2 --addr 127.0.0.1:47822
```

Tell the CLI to use the same address:

```powershell
$env:AMIGO_CODEMAP_DAEMON_ADDR = "127.0.0.1:47822"
```

Remove the setting:

```powershell
Remove-Item Env:\AMIGO_CODEMAP_DAEMON_ADDR
```

## Recommended daily workflow

Start daemon once:

```powershell
.\target\debug\amigo-codemapd.exe run --root D:\Git\amigo --level 2
```

Use CLI normally in another terminal:

```powershell
.\target\debug\amigo-codemap.exe brief
.\target\debug\amigo-codemap.exe trace EditorTarget --limit 20
.\target\debug\amigo-codemap.exe change-plan "right dock target context"
```

Stop daemon when finished:

```powershell
.\target\debug\amigo-codemapd.exe shutdown
```

## Troubleshooting

### Daemon does not start

Check if the port is already used:

```powershell
netstat -ano | findstr 47821
```

If an old daemon is running, stop it:

```powershell
.\target\debug\amigo-codemapd.exe shutdown
```

Or kill the process manually by PID:

```powershell
Stop-Process -Id <PID> -Force
```

### CLI is still slow

Enable diagnostics:

```powershell
$env:AMIGO_CODEMAP_DAEMON_VERBOSE = "1"
.\target\debug\amigo-codemap.exe brief
```

Possible causes:

```text
- daemon is not running
- daemon address mismatch
- root path mismatch
- daemon marked snapshot dirty and refreshed the map
- command still uses a heavy path not yet daemon-optimized
```

### Root mismatch

The daemon is workspace-specific.

If daemon was started for another root, stop it:

```powershell
.\target\debug\amigo-codemapd.exe shutdown
```

Then restart it with the correct root:

```powershell
.\target\debug\amigo-codemapd.exe run --root D:\Git\amigo --level 2
```

### Force old behavior

```powershell
$env:AMIGO_CODEMAP_NO_DAEMON = "1"
.\target\debug\amigo-codemap.exe brief
Remove-Item Env:\AMIGO_CODEMAP_NO_DAEMON
```

## Current limitations

The first daemon version keeps a full codemap snapshot in memory and refreshes it when the watcher marks the workspace dirty.

It is not yet a fully incremental symbol indexer.

Expected next improvements:

```text
1. Re-index only changed files.
2. Keep file hashes and per-file symbol data in memory.
3. Rebuild reference graph lazily.
4. Add operation validation through daemon.
5. Add apply/preview operations through daemon.
6. Add optional tray/status application.
```

## Target architecture

```text
amigo-codemap.exe
  CLI client
  tries daemon first
  falls back to local snapshot/cache

amigo-codemapd.exe
  local workspace daemon
  keeps codemap in RAM
  watches files
  refreshes dirty snapshot
  serves local JSON requests

future amigo-tray.exe
  optional UI client
  shows daemon status
  can trigger rescan/shutdown
```

The daemon is not a replacement for validation.
Mutating operations should still validate file hash, context, anchors, and expected ranges before writing files.
