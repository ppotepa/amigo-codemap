# amigo-codemap 0.1 Documentation

`amigo-codemap` is the operational navigation and planning layer for the Amigo repository. It helps humans and LLM agents answer "where is this?", "what does this affect?", "what should I read?", and "how can I safely apply this change?" before opening large files or running broad text searches.

It does not replace the compiler, tests, or code review. It replaces a large part of the repetitive discovery work usually done with `rg`, manual file browsing, and ad-hoc patch planning.

## Benchmark Showcase

These benchmarks compare codemap-first discovery against a focused standard workflow using `rg` and targeted `Get-Content`. The measurement covers research output only: commands, files opened, lines read, and estimated context tokens before the first correct patch. Full protocol and commands are documented later in [Codemap Workflow Benchmark Protocol](#codemap-workflow-benchmark-protocol).

### Benchmark 1: Snapshot Label Passthrough

| Method | Commands | Files opened | Lines read | Est. tokens | Result |
|---|---:|---:|---:|---:|---|
| codemap-first | 7 | 2 | 25 | 941 | pass |
| standard | 5 | 4 | 2159 | 15874 | pass |

Codemap-first used ~94.1% fewer estimated context tokens by reading two symbol slices instead of full DTO/source files.

### Benchmark 2: Real-Snapshot Guard

| Method | Commands | Files opened | Lines read | Est. tokens | Result |
|---|---:|---:|---:|---:|---|
| codemap-first | 10 | 5 | 230 | 3759 | pass |
| standard | 9 | 8 | 3509 | 38561 | pass |

Codemap-first used ~90.2% fewer estimated context tokens. `symbols --file --metadata` also corrected a stale guessed backend symbol to the actual `fallback_editor_snapshot`.

### Benchmark 3: Pointer Fast-Path

| Method | Commands | Files opened | Lines read | Est. tokens | Result |
|---|---:|---:|---:|---:|---|
| codemap-first | 13 | 7 | 822 | 14210 | pass |
| standard | 14 | 13 | 4775 | 79403 | pass |

Codemap-first used ~82.1% fewer estimated context tokens by using symbol metadata and narrow slices instead of full Tauri/editor-mode files.

Short version: codemap does not mainly reduce command count. It reduces how much code the agent must read before it can safely edit.

## Quickstart

Daemon usage guide is available in:

```text
README_DAEMON.md
```

Build the tool:

```powershell
cargo build -p amigo-codemap
```

Optionally create a short alias for the current PowerShell session:

```powershell
Copy-Item target\debug\amigo-codemap.exe target\debug\amigo-codemap-stable.exe
$cm = "target\debug\amigo-codemap-stable.exe"
```

Run the default navigation loop:

```powershell
& $cm brief
& $cm changes --compact --hide-generated --limit 20
& $cm trace patch-apply --limit 20
& $cm open-set patch-apply --why --limit 10
& $cm impact patch-apply --limit 30
& $cm verify-plan --changed
```

For most tasks, start with:

```powershell
& $cm change-plan <query> --limit 20
& $cm trace <thing> --limit 20
& $cm open-set <thing> --why --limit 10
```

## Raw Ops Workflow

Use codemap as the first navigation layer. Do not begin implementation by reading whole files, running repo-wide `rg`, or opening `concat-output.txt`.

For chat-facing change instructions, prefer raw ops blocks:

```text
ACTION: REPLACE SYMBOL
FILE: crates/example/src/lib.rs
SYMBOL: run
CONTENT:
fn run() {}
END
```

`ops-preview`, `ops-check`, `ops-apply`, `ops-verify`, `ops-summary`, and `ops-skeleton` accept `--raw`. Internally the tool converts raw blocks to the existing `OpsPlan` model, so YAML remains the storage/interchange backend while raw stays the preferred human/agent format.

## Fast Snapshot Workflow

Most report commands read the fast snapshot cache from:

```text
.amigo/codemap.snapshot.json
```

The first report command may scan the repository and create the cache. Later commands should be much faster.

Refresh manually:

```powershell
& $cm refresh
```

Check status:

```powershell
& $cm status
```

Keep the snapshot updated in a separate terminal:

```powershell
& $cm watch --write
```

Then use regular commands in another terminal:

```powershell
& $cm trace ui-document
& $cm open-set ui-document --why --limit 10
& $cm impact ui-document
```

Force a full scan when debugging scanner behavior or stale cache suspicion:

```powershell
& $cm trace ui-document --no-cache
```

## Live Git Change Summary

`changed` is snapshot-aware and can be stale if the fast cache is stale. For live working tree state, use:

```powershell
& $cm changes --compact
& $cm changes --compact --hide-generated
& $cm changes --group domain
& $cm changes --warnings
& $cm commit-plan --compact
```

Use `changes` instead of manual `git status --short` and `git diff --stat` in normal agent workflow. It reads live git state, groups generated files and submodules, prints a compact shortstat, and suggests next commands.

Use `commit-plan` before committing when changes need to be split into logical commits.

## Codemap Taxonomy And Anchors

Codemap anchors are stable navigation points in the repository.

Human-readable taxonomy:

- `codemap.index.md`

Machine-readable taxonomy:

- `.amigo/codemap.taxonomy.yml`

Generated anchor data:

- `.amigo/codemap.anchors.generated.json`
- `.amigo/codemap.coverage.generated.md`

Common commands:

```powershell
& $cm taxonomy
& $cm anchors priority:P0 --limit 20
& $cm anchors domain:ui-document --limit 20
& $cm anchors --write
& $cm anchor-check
```

Anchors are integrated with navigation reports:

```powershell
& $cm trace editor-dock-registry
& $cm open-set ui-document --why --limit 10
& $cm change-plan scene-editor --limit 20
& $cm neighbors crates/tools/amigo-codemap/src/report/tauri_graph.rs
```

P0/P1 anchors should be manual and meaningful. P2 anchors may be generated file-level coverage anchors.

### Maintaining Anchors During Feature Work

`amigo-codemap` is a living repository index, not a one-time report.

When a feature adds or moves important engine, editor, runtime, backend, or mod surfaces, update codemap in the same change:

1. Add manual P0/P1 anchors for new entrypoints, dispatchers, registries, root models, DTO contracts, command handlers, editor roots, scene YAML files, and scene scripts.
2. Update `.amigo/codemap.taxonomy.yml` when the feature introduces a new domain, role, layer, or scoring rule.
3. Regenerate generated files:

```powershell
& $cm anchors --write
& $cm anchor-check
```

4. Commit the feature code together with the updated taxonomy/index files:

```text
codemap.index.md
.amigo/codemap.taxonomy.yml
.amigo/codemap.anchors.generated.json
.amigo/codemap.coverage.generated.md
```

Generated P2 anchors provide broad coverage. Manual P0/P1 anchors explain intent and should exist for places an agent should open early.

## What amigo-codemap Is

`amigo-codemap` builds and reads an operational snapshot of the repository. The snapshot contains files, file tags, symbols, symbol metadata, text occurrences, relationships, git state, and command/workflow hints.

The goal is simple:

```text
Do less searching.
Open fewer files.
Read smaller slices.
Plan changes before editing.
Verify changes predictably.
```

A typical old workflow looks like this:

```powershell
rg "SelectionProperties"
Get-Content crates/apps/amigo-editor/src/properties/SelectionProperties.tsx
rg "SelectionProperties" crates/apps/amigo-editor/src
rg "entity.inspector"
rg "invoke"
```

A codemap-first workflow looks like this:

```powershell
& $cm where SelectionProperties
& $cm signature SelectionProperties
& $cm trace entity.inspector
& $cm open-set entity.inspector --why
& $cm impact SelectionProperties
```

The important difference: `codemap` tries to explain structure and next steps, not just dump matching text.

## Mental Model

### Snapshot

The snapshot is the indexed view of the repository. It stores enough metadata for navigation without opening source files first.

Typical snapshot data:

```text
files
symbols
text occurrences
relations
tags
git state
packages
areas
```

### File

A file entry contains path, language, line count, hash, size, tags, and git state.

Example file tags:

```text
layer:app
layer:tool
layer:engine
kind:source
kind:test
kind:config
domain:workspace
domain:codemap
state:changed
risk:large
```

### Symbol

A symbol is a structural code item, for example a function, method, component, hook, struct, enum, interface, type, const, module, Rhai function, YAML key-like symbol, or CSS selector.

A symbol entry should know:

```text
name
kind
file
line range
signature
params
return type
generics
visibility
owner
tags
confidence
```

### Text Occurrence

A text occurrence is not necessarily a symbol. It is a meaningful string, key, ID, selector, or config value.

Examples:

```text
"entity.inspector"
"send_editor_pointer_event"
"scene.yml"
"main-menu"
".workspace-panel"
```

Text occurrences matter because modern app logic often connects through IDs, config, scene files, command names, CSS classes, or YAML values.

### Relation

A relation describes a connection:

```text
file imports file
symbol references symbol
frontend invokes backend command
scene references asset
component uses component
```

0.1 relations are heuristic navigation hints, not compiler truth.

### Anchor

An anchor is a deliberate marker for codemap navigation.

```ts
// @codemap anchor:workspace-dock domain:workspace role:registry
```

```rust
// @codemap anchor:editor-mode-pointer domain:editor-mode role:command
```

```yaml
# @codemap anchor:main-menu-scene domain:menu role:scene
```

Use anchors for important registries, command maps, pipelines, central dispatchers, or places that are hard to discover automatically.

### Workset

A workset is a saved scope of work: files, symbols, reasons, risks, and verification commands. It prevents repeated discovery across multiple edits.

```powershell
& $cm workset ui-document-inspector --from-impact UiDocumentEditor --save
& $cm workset ui-document-inspector --status
```

## What Codemap Replaces

| Old workflow | Problem | Codemap command | Benefit |
| --- | --- | --- | --- |
| `rg "Foo"` | Raw matches, no meaning | `where Foo` | Finds definitions and references |
| Open full file | Expensive context | `slice <file> --symbol Foo` | Reads only the relevant symbol |
| `rg "entity.inspector"` | Hard to know what the string means | `trace entity.inspector` | Classifies string/id/config usage |
| Manually inspect imports | Slow and noisy | `neighbors <file>` | Shows nearby related files |
| Guess files to read | Over-opens context | `open-set <query> --why` | Ranked file list with reasons |
| Guess breakage | Missed callsites | `impact <query>` | Shows direct/text/config impact |
| Guess tests | Inconsistent verification | `verify-plan --changed` | Suggests verification steps |
| Manual patching | Fragile edits | `patch-check` / `ops-check` | Validates before writing |
| Huge diff in prompt | Token-heavy | `ops-*` | Compact declarative operations |
| Repeat research each turn | Wasted tokens | `workset` | Saves task scope |

## Recommended Daily Loop

Use this for most coding tasks:

```powershell
cargo build -p amigo-codemap

$cm = "target\debug\amigo-codemap.exe"

& $cm changed --group package --limit 20
& $cm change-plan <query> --limit 20
& $cm trace <thing> --limit 20
& $cm open-set <thing> --why --limit 10
& $cm signature <symbol>
& $cm slice <file> --symbol <symbol>
& $cm impact <symbol-or-query> --limit 30
& $cm verify-plan --changed
```

### Step 1: Check Changed Scope

```powershell
& $cm changed --group package --limit 20
```

Use when starting a new task, continuing after patches, or checking what is dirty.

### Step 2: Build A Task Plan

```powershell
& $cm change-plan ui-document-inspector --limit 20
```

Expected sections:

```text
scope
symbols
text/config
suggested commands
verify
```

### Step 3: Trace Unknown Things

```powershell
& $cm trace entity.inspector --limit 20
```

Use when the query may be a symbol, string literal, dock ID, scene ID, asset ID, CSS class, Tauri command, Rhai function, YAML key, or YAML value.

### Step 4: Get A Ranked Read List

```powershell
& $cm open-set entity.inspector --why --limit 10
```

Run this before opening files. Treat the reasons as ranking hints, not proof.

### Step 5: Inspect A Symbol Without Opening The File

```powershell
& $cm signature SelectionProperties
```

Use when you need signature, parameters, return type, generics, visibility, owner, and line range.

### Step 6: Read Only The Relevant Symbol

```powershell
& $cm slice crates/apps/amigo-editor/src/properties/SelectionProperties.tsx --symbol SelectionProperties
```

Use when the file is large or only one function/method/component matters.

`slice --symbol` accepts exact names and normalized names such as camelCase vs snake_case. If a symbol is not found, it prints nearby symbols from the same file with similarity scores and suggests retrying with one of them.

### Step 7: Check Impact

```powershell
& $cm impact SelectionProperties --limit 30
```

Expected sections:

```text
direct impact
text/config impact
likely affected
tests
verify
```

### Step 8: Verify

```powershell
& $cm verify-plan --changed
```

Then run the suggested build and tests. Compiler/tests remain final truth.

## Problem To Command

| Problem | Command | PowerShell example | Expected result |
| --- | --- | --- | --- |
| I have a symbol name and need its definition | `where` | `& $cm where CodeMap` | Definitions, line ranges, references |
| I need the type/signature | `signature` | `& $cm signature scan_symbols` | Params, return type, owner, visibility |
| I have a string or ID | `trace` | `& $cm trace entity.inspector` | Meaning, occurrences, related files |
| I do not know which files to open | `open-set --why` | `& $cm open-set ui-document --why` | Ranked files with reasons |
| I need the method list from one file | `symbols --file --metadata` | `& $cm symbols --file src/foo.rs --metadata` | Symbols, ranges, params, returns, tags |
| I want only one method/component | `slice --symbol` | `& $cm slice src/foo.rs --symbol parse` | Only that symbol's code |
| I want to know what may break | `impact` | `& $cm impact send_editor_pointer_event` | Direct and likely affected areas |
| I want a full task plan | `change-plan` | `& $cm change-plan editor-mode-pointer` | Scope, symbols, next commands, verify |
| I want related files | `neighbors` | `& $cm neighbors src/main.rs` | Imports, relations, same-domain files |
| I want to understand a file | `explain-file` | `& $cm explain-file src/main.rs` | File role, tags, symbols, occurrences |
| I want public/export API | `api-surface` | `& $cm api-surface --limit 50` | Public/export symbols |
| I want TSX component overview | `component-graph` | `& $cm component-graph --limit 50` | Components, props/signatures |
| I want frontend/backend Tauri flow | `tauri-graph` | `& $cm tauri-graph --limit 50` | Invokes, backend commands, DTO hints |
| I want likely callsites | `callsite-candidates` | `& $cm callsite-candidates scan_symbols` | Heuristic callsite list |
| I want TODO/risk scope | `todo-index` / `risk-index` | `& $cm risk-index --limit 30` | Indexed TODO/risk/large/changed files |
| I want refactor candidates | `smells` | `& $cm smells --top 30 --why` | Ranked Refactor Radar with score, smells, metrics, and next actions |
| I want to apply changes safely | `patch-check` / `ops-check` | `& $cm ops-check --from plan.yml` | Validates before write |
| I want ops YAML schema | `ops-schema` | `& $cm ops-schema --example replace_symbol` | Required/optional fields and examples |
| I want an ops plan starter | `ops-skeleton` | `& $cm ops-skeleton scan_symbols --out plan.yml --write` | Creates a YAML operations skeleton |
| I need stable symbol range data | `range-for-symbol` | `& $cm range-for-symbol scan_symbols` | Path, lines, hash, signature, ops hint |
| I need stable line range YAML | `range-for-lines` | `& $cm range-for-lines src/foo.ts 10 20 --yaml-op replace_range` | Safe range op with hash and context |
| I need stable anchor range data | `anchor-range` | `& $cm anchor-range properties-registry` | Path, lines, hash, anchor locator |
| I want verify commands from YAML | `ops-verify` | `& $cm ops-verify --from plan.yml` | Prints plan verify commands |
| I want operations log text | `ops-summary` | `& $cm ops-summary --from plan.yml --changed` | operations.md-ready summary |
| I want to save scope | `workset` | `& $cm workset ui-doc --from-impact UiDocumentEditor --save` | Saved task context |

## Command Reference

### `brief`

Show a compact overview of the repository snapshot.

```powershell
& $cm brief
```

Use when starting work, checking whether codemap sees the repo, or orienting a new agent.

### `smells`

Print the Refactor Radar / Code Smell Index. This is a ranking for likely technical debt hotspots, not a build-blocking lint.

```powershell
& $cm smells --top 30 --why
& $cm smells --changed --why
& $cm smells --file crates/tools/amigo-codemap/src/report/file_ops/ops_plan/mod.rs --why
& $cm smells --group domain --top 30
& $cm smells --json
```

Use `--report` for a full report instead of a top-N list. Use `--report-file` to save directly.

```powershell
& $cm smells --report --report-file .amigo/code-smells-report.json --file-lines 500 --min-score 30 --why
```

Useful options:

```text
--top N
--changed
--group domain|package|path|language|smell|severity
--min-score N
--file-lines N
--json
--why
--include-tests
--include-generated
--file <path>
--report
--report-file <path>
``` 

`--file-lines N` controls file-size smells:

```text
file.large     > N lines
file.too_large > N + 250 lines
file.god_file  > N + 550 lines
```

### `scan`

Build or print the codemap snapshot.

```powershell
& $cm scan --level 2 --pretty
```

Recommended levels:

```text
level 0: files only
level 1: public/export symbols
level 2: symbols + text occurrences
level 3: deeper relation/report context
```

### `changes`

Show live git working tree status with compact grouping and shortstat. This is the preferred command for current dirty state.

```powershell
& $cm changes --compact --hide-generated
& $cm changes --group domain
& $cm changes --warnings
```

Typical next commands:

```powershell
& $cm commit-plan --compact
& $cm verify-plan --changed
```

### `changed`

Show changed files from the current codemap snapshot. This can be stale if `watch --write` is not running or `refresh` has not been run recently.

```powershell
& $cm changed --group package --limit 20
```

Typical next commands:

```powershell
& $cm open-set <changed-area> --why
& $cm verify-plan --changed
```

### `files`

Filter files by tags, language, domain, layer, and state.

```powershell
& $cm files --query layer:app,kind:source --limit 30
& $cm files --query domain:workspace,state:changed --limit 30
& $cm files --query lang:tsx,!kind:test --limit 30
```

Common query tags:

```text
layer:app
layer:tool
kind:source
kind:test
domain:workspace
domain:codemap
state:changed
lang:rs
lang:tsx
!kind:test
```

### `symbols`

List symbols with metadata.

```powershell
& $cm symbols --query name:CodeMap --limit 20
& $cm symbols --query kind:component,visibility:export --limit 20
& $cm symbols --query lang:rs,kind:fn --limit 20
& $cm symbols --file crates/tools/amigo-codemap/src/scan/symbols.rs --metadata --limit 20
```

Use `--file <path>` to list symbols from one file. Use `--metadata` when you need params, return type, generics, owner, tags, confidence, and line ranges.

Recommended workflow for a large file:

```powershell
& $cm symbols --file crates/tools/amigo-codemap/src/scan/symbols.rs --metadata --limit 40
& $cm slice crates/tools/amigo-codemap/src/scan/symbols.rs --symbol build_symbol
```

Example metadata output:

```text
fn build_symbol
  file: crates/tools/amigo-codemap/src/scan/symbols.rs
  range: 233-269 (37 lines)
  visibility: local
  owner: -
  confidence: 75
  params: file: &FileEntry, name: String, kind: String, line: usize, visibility: String, owner: Option<String>, extracted: ExtractedSignature, mut tags: Vec<String>
  returns: SymbolEntry
  generics: -
  tags: domain:codemap, ext:rs, kind:fn, kind:source, lang:rs, layer:tool, risk:large, state:clean, visibility:local
  signature: fn build_symbol(...) -> SymbolEntry
```

Language behavior:

```text
Rust/TS/TSX: functions, methods, types, components, params, returns, generics where detected.
Rhai: functions with lightweight signatures.
YAML: key-like symbols; params/returns are usually empty.
CSS: selectors as symbols; params/returns are empty.
JSON/TOML/Markdown: mostly text/config navigation via trace and explain-file, not rich symbols.
```

### `where`

Find where a symbol lives and where it is referenced.

```powershell
& $cm where SelectionProperties --limit 10
```

Use this instead of raw `rg` when the query is a known symbol.

### `signature`

Show signature and metadata for a symbol without opening the file.

```powershell
& $cm signature scan_symbols
```

Typical output includes:

```text
file
range
visibility
owner
params
returns
generics
tags
signature
```

For 0.1, signatures are heuristic for complex declarations, but they are good enough to avoid many full-file reads.

### `trace`

Trace a symbol, string, ID, command, scene, asset, CSS class, or config value.

```powershell
& $cm trace entity.inspector --limit 20
```

Expected sections:

```text
matched symbols
matched text occurrences
matched anchors
likely meaning
related files
next
```

### `open-set`

Suggest which files to open, in order.

```powershell
& $cm open-set entity.inspector --why --limit 10
```

The `--why` flag prints scoring reasons such as symbol match, text occurrence, anchor, changed state, domain tag, or risk tag.

### `slice`

Print only a portion of a file.

```powershell
& $cm slice crates/tools/amigo-codemap/src/scan/symbols.rs --symbol scan_symbols
```

Use after `signature` or `open-set --why`.

### `impact`

Estimate what may be affected by a change.

```powershell
& $cm impact SelectionProperties --limit 30
```

Expected sections:

```text
direct impact
text/config impact
likely affected
tests
verify
```

### `verify-plan`

Suggest verification commands based on changed files.

```powershell
& $cm verify-plan --changed
```

Always run compiler/tests. Codemap is a planner, not final truth.

### `workset`

Save and inspect task scope.

```powershell
& $cm workset ui-document-inspector --from-impact UiDocumentEditor --save
& $cm workset ui-document-inspector --status
```

Use when a task spans multiple turns or when you want to preserve why a file matters.

## Patch And Ops Workflows

### Unified Diff

Use these when you already have a normal patch/diff:

```powershell
& $cm patch-preview --from patch.diff
& $cm patch-check --from patch.diff
& $cm patch-apply --from patch.diff --write
```

### Declarative Ops

Use ops when a change is better described as file operations:

```text
create this file
replace this line range
delete this range
insert after this anchor
replace this symbol
replace this method body
copy or move this file
load replacement code from a sidecar file
```

Always run:

```powershell
& $cm ops-skeleton <query> --out plan.yml --write
& $cm ops-schema --example replace_symbol
& $cm ops-preview --from plan.yml
& $cm ops-check --from plan.yml --strict
& $cm ops-apply --from plan.yml --write --backup --stop-on-error
& $cm ops-verify --from plan.yml
& $cm ops-summary --from plan.yml --changed
```

Use `range-for-symbol` before hand-writing symbol/range based plans:

```powershell
& $cm range-for-symbol scan_symbols
```

Use `range-for-lines` when instructions are line-based and should become safe YAML ops:

```powershell
& $cm range-for-lines src/foo.ts 10 20 --yaml-op replace_range
& $cm range-for-lines src/foo.ts 10 20 --yaml-op delete_range
```

Use `anchor-range` before hand-writing anchor based plans:

```powershell
& $cm anchor-range properties-registry
& $cm anchor-range properties-registry-start --to properties-registry-end
```

Ops input can come from a file, stdin, or inline YAML:

```powershell
& $cm ops-check --from plan.yml
Get-Content .\plan.yml | & $cm ops-check --from -
$yaml = "task: inline`nops: []`n"
& $cm ops-check --yaml $yaml
```

`version: 1` is optional for ops plans. If it is omitted, codemap treats the
plan as ops-plan v1. Prefer the shorter `ops:` form for small plans.

For larger changes, keep YAML as control data and put code in sidecar files. `content_from`
is resolved relative to the plan file, or relative to `content_root` under the plan file
directory when `content_root` is set:

```text
.amigo/ops/my-task/
  plan.yml
  updates/
    NewPanel.tsx
    replacement.ts
```

```yaml
task: my-task
content_root: updates
ops:
  - id: create-panel
    kind: create_file
    path: crates/apps/amigo-editor/src/features/example/NewPanel.tsx
    content_from: NewPanel.tsx

  - id: replace-range
    kind: replace_range
    path: crates/apps/amigo-editor/src/features/example/Existing.tsx
    start_line: 20
    end_line: 40
    expected_hash: "abc12345"
    content_from: replacement.ts
```

Use exactly one of `content`/`replace` or `content_from` for content-bearing operations.
All operation paths must be repo-relative; absolute paths and `..` are rejected.

Expected output includes:

```text
path: crates/tools/amigo-codemap/src/scan/symbols.rs
start_line: 11
end_line: 29
hash: 11d3d664
expected_hash: 11d3d664
signature: pub fn scan_symbols(...)
```

Stable operations for 0.1:

```text
create_file
replace_file
delete_file
copy_file
move_file
rename_file
create_dir
delete_dir
append_to_file
replace_range
delete_range
insert_before_anchor
insert_after_anchor
replace_between_anchors
```

Symbol-aware operations are implemented but should be treated as experimental until smoke-tested on the target file:

```text
replace_symbol
delete_symbol
insert_before_symbol
insert_after_symbol
replace_method_body
```

Safety priority:

```text
1. symbol + expected_hash
2. anchor + context
3. replace_between_anchors
4. range + expected_hash + context_before/context_after
5. replace_file
```

`ops-check --strict` fails unsafe plans more aggressively. In strict mode, range ops should include `expected_hash` and context, symbol ops must resolve uniquely, and missing anchors/symbols are hard failures.

`ops-check` validates a plan as a sequence for basic file-system ops, so a later `copy_file` may refer to a file created earlier in the same plan.

`ops-apply` is dry-run by default. It writes only with `--write`. Use `--backup` to copy touched files to `.amigo/ops-backups/<task>/...` before writes and `--stop-on-error` to avoid continuing after a failed op. `ops-apply --strict` reuses strict validation, applies all ops regardless of `--limit`, and exits non-zero when any operation fails.

### Ops Schema

Use this when writing or reviewing YAML:

```powershell
& $cm ops-schema
& $cm ops-schema --json
& $cm ops-schema --example replace_symbol
```

Typical output:

```yaml
kind: replace_symbol
required:
  - path
  - symbol
optional:
  - id
  - content
  - content_from
  - expected_hash
  - context_before
  - context_after
```

### Ops Split, Verify, Summary

Use these helpers after a larger plan exists:

```powershell
& $cm ops-split --from all-tree-migration.yml --by domain
& $cm ops-split --from all-tree-migration.yml --by risk
& $cm ops-verify --from 001-shared-tree.yml
& $cm ops-summary --from 001-shared-tree.yml --changed
```

`ops-split` currently proposes logical split files; it does not write split YAMLs. `ops-verify --run` is intentionally conservative in 0.1 and prints commands instead of silently executing arbitrary verify steps.

### Example: Replace A Line Range

```yaml
task: replace-one-line
description: "Replace one line in a temp file."
ops:
  - id: replace-b-line
    kind: replace_range
    path: tmp.txt
    start_line: 2
    end_line: 2
    expected_hash: "11d3d664"
    context_before: "a"
    context_after: "c"
    content: |
      B
verify:
  - Get-Content .\tmp.txt
```

### Example: Create A File

```yaml
content_root: updates
ops:
  - id: create-new-panel
    kind: create_file
    path: crates/apps/amigo-editor/src/example/NewPanel.tsx
    content_from: NewPanel.tsx
```

### Example: Copy And Move Files

```yaml
task: reorganize-panels
ops:
  - id: copy-panel
    kind: copy_file
    from: crates/apps/amigo-editor/src/example/Panel.tsx
    to: crates/apps/amigo-editor/src/example/copied/Panel.tsx
    expected_hash: "abc12345"
    overwrite: false

  - id: move-panel
    kind: move_file
    from: crates/apps/amigo-editor/src/example/OldPanel.tsx
    to: crates/apps/amigo-editor/src/example/NewPanel.tsx
    expected_hash: "def67890"
    overwrite: false
```

### Example: Insert After Anchor

```yaml
ops:
  - id: add-properties-panel-import
    kind: insert_after_anchor
    path: crates/apps/amigo-editor/src/properties/propertiesRegistry.tsx
    anchor: "// @codemap anchor:properties-registry domain:properties role:registry"
    content: |
      import { UiDocumentPropertiesPanel } from "./panels/UiDocumentPropertiesPanel";
```

### Example: Replace Between Anchors

```yaml
task: replace-registry-section
ops:
  - id: replace-properties-registry-section
    kind: replace_between_anchors
    path: crates/apps/amigo-editor/src/properties/propertiesRegistry.tsx
    start_anchor: "// @codemap anchor:properties-registry-start domain:properties role:registry"
    end_anchor: "// @codemap anchor:properties-registry-end domain:properties role:registry"
    expected_hash: "11d3d664"
    content: |
      import { UiDocumentPropertiesPanel } from "./panels/UiDocumentPropertiesPanel";
```

`replace_between_anchors` keeps both anchor lines and replaces only the content between them.

### Ops-First Response Format

For medium and large changes, prefer this structure:

```text
Task:
  short-name

Intent:
  what this change fixes

Codemap:
  commands used to narrow scope

Files:
  files touched

Ops:
  plan.yml

Verify:
  commands to run

Acceptance:
  final conditions
```

The code changes should live in `plan.yml` whenever practical. Prose should explain intent and risks, not duplicate the patch.

### Example: Replace Symbol, Experimental

```yaml
ops:
  - id: replace-scan-symbols
    kind: replace_symbol
    path: crates/tools/amigo-codemap/src/scan/symbols.rs
    symbol: scan_symbols
    expected_hash: "11d3d664"
    content: |
      pub fn scan_symbols(...) -> Result<Vec<SymbolEntry>> {
          todo!("new implementation")
      }
```

Before symbol-aware ops, run:

```powershell
& $cm signature scan_symbols
& $cm slice crates/tools/amigo-codemap/src/scan/symbols.rs --symbol scan_symbols
& $cm range-for-symbol scan_symbols
& $cm ops-check --from plan.yml --strict
```

Prefer, in order:

```text
symbol-aware operation, if stable
anchor
context_before/context_after
expected_hash
line range
```

## `@codemap` Anchors

Anchors are explicit navigation markers.

TypeScript / TSX:

```ts
// @codemap anchor:workspace-dock domain:workspace role:registry
```

Rust:

```rust
// @codemap anchor:editor-mode-pointer domain:editor-mode role:command
```

YAML:

```yaml
# @codemap anchor:main-menu-scene domain:menu role:scene
```

CSS:

```css
/* @codemap anchor:scene-editor-layout domain:scene-editor role:style */
```

Add anchors to central registries, large dispatchers, command maps, Tauri command registration, dock/component registries, scene transition points, important YAML scenes, generated/hand-maintained boundaries, and high-risk files.

Do not add anchors to every function, obvious local helpers, temporary code, small files, or normal comments that already explain intent.

## Language Support

| Language / format | File indexing | Symbols | Text occurrences | Relations | Typical use |
| --- | ---: | ---: | ---: | ---: | --- |
| Rust `rs` | yes | strong | yes | `mod/use` heuristic | backend, tools, engine, Tauri |
| TypeScript `ts` | yes | strong | yes | imports | API, services, app logic |
| TSX `tsx` | yes | strong | yes | imports/components heuristic | React components, docks, panels |
| Rhai `rhai` | yes | functions | yes | light | scripts, scene behavior |
| YAML/YML | yes | keys/scenes | strong | config refs | scenes, assets, prefabs |
| TOML | yes | limited | yes | package/config | mod metadata, Cargo |
| CSS | yes | selectors | strong | class refs heuristic | styles and class tracing |
| JSON | yes | limited | yes | config | generated/config data |
| Markdown | yes | limited | limited | docs refs | docs and planning |
| HTML | yes | limited | ids/classes | light | app shell |

0.1 is strongest for Rust, TypeScript, TSX, YAML, and Rhai trace workflows.

## Scenarios

### Change A UI Component But You Do Not Know Where To Start

```powershell
& $cm change-plan ui-document-inspector --limit 20
& $cm trace ui-document-inspector --limit 20
& $cm open-set ui-document-inspector --why --limit 10
& $cm signature UiDocumentEditor
& $cm slice crates/apps/amigo-editor/src/features/ui/UiDocumentEditor.tsx --symbol UiDocumentEditor
& $cm impact UiDocumentEditor --limit 30
& $cm verify-plan --changed
```

### You Found `entity.inspector`

```powershell
& $cm trace entity.inspector --limit 20
& $cm open-set entity.inspector --why --limit 10
& $cm impact entity.inspector --limit 30
```

### Change A Tauri Command

```powershell
& $cm trace send_editor_pointer_event --limit 20
& $cm tauri-graph --limit 50
& $cm where send_editor_pointer_event
& $cm impact send_editor_pointer_event --limit 30
& $cm verify-plan --changed
```

### Read Only One Method From A Large File

```powershell
& $cm symbols --file crates/tools/amigo-codemap/src/scan/symbols.rs --metadata --limit 40
& $cm signature scan_symbols
& $cm slice crates/tools/amigo-codemap/src/scan/symbols.rs --symbol scan_symbols
```

### Apply A Prepared Change Plan

```powershell
& $cm ops-preview --from .\plan.yml
& $cm ops-check --from .\plan.yml
& $cm ops-apply --from .\plan.yml --write
& $cm verify-plan --changed
```

Then run actual verification:

```powershell
cargo fmt -p amigo-codemap
cargo test -p amigo-codemap
cargo build -p amigo-codemap
```

## Token Savings

Codemap reduces token usage by reducing uncertainty.

| Operation | Old token cost | Codemap approach | Effect |
| --- | --- | --- | --- |
| Find symbol | `rg` + multiple files | `where` / `signature` | Often no file read needed |
| List methods in one file | full file read | `symbols --file --metadata` | Pick a symbol before reading code |
| Understand string/id | raw `rg` output | `trace` | Meaning and related files |
| Choose files | manual guessing | `open-set --why` | Open fewer files |
| Read implementation | full file | `slice --symbol` | Read only target code |
| Check impact | repeated search | `impact` | One structured report |
| Apply changes | large diff in chat | `ops-*` | Compact operation format |
| Continue task | repeat discovery | `workset` | Persistent scope |
| Verify | guess tests | `verify-plan` | Consistent commands |

Example:

```text
Old: open 8 files, each 300 lines = 2400 lines of context.
New: trace -> open-set --why -> signature -> slice = 2 symbol slices, often under 100 lines.
```

## Stable Output Contract

Reports should use predictable sections:

```text
scope
findings
definitions
references
text/config
related files
risks
next
verify
```

The `next:` section is important. It tells the agent what command to run next without another discovery pass.

## Limitations In 0.1

`amigo-codemap 0.1` is useful, but it is not magic.

```text
It is not an LSP.
It does not replace rustc, TypeScript, tests, or code review.
Some parsing is heuristic.
Callsite candidates are not a full call graph.
TSX component graph may be incomplete.
Tauri graph may be heuristic.
YAML/Rhai support is intentionally lighter than Rust/TS/TSX.
Snapshot data can become stale.
Line-based operations can be fragile.
Symbol-aware operations should be treated as experimental unless validated by tests.
```

Always finish with compiler/tests.

## Checklists

### Before Changing Code

```powershell
& $cm changed --group package --limit 20
& $cm change-plan <query> --limit 20
& $cm trace <thing> --limit 20
& $cm open-set <thing> --why --limit 10
& $cm impact <thing> --limit 30
```

### Before Opening A Large File

```powershell
& $cm symbols --file <file> --metadata --limit 40
& $cm signature <symbol>
& $cm slice <file> --symbol <symbol>
```

### Before Applying A Patch

```powershell
& $cm patch-preview --from patch.diff
& $cm patch-check --from patch.diff
```

### Before Applying Ops

```powershell
& $cm ops-preview --from plan.yml
& $cm ops-check --from plan.yml
```

### After Changes

```powershell
cargo fmt -p amigo-codemap
cargo test -p amigo-codemap
cargo build -p amigo-codemap

& $cm verify-plan --changed
```

## Codemap Workflow Benchmark Protocol

Use this benchmark when validating whether `amigo-codemap` saves agent context compared with a conventional `rg`/`Get-Content` workflow.

The benchmark is intentionally practical, not scientific. The main question is:

```text
How many files and lines must the agent inspect before the first correct patch?
```

### Showcase Results

These results compare codemap-first discovery against a focused standard workflow using `rg` and targeted `Get-Content`. The benchmark measured research output only: commands, files opened, lines read, and estimated context tokens before making the first correct patch.

| Task | Method | Commands | Files opened | Lines read | Est. tokens | Token saving |
|---|---|---:|---:|---:|---:|---:|
| Snapshot label passthrough | codemap-first | 7 | 2 | 25 | 941 | 94.1% |
| Snapshot label passthrough | standard | 5 | 4 | 2159 | 15874 | baseline |
| Real-snapshot guard | codemap-first | 10 | 5 | 230 | 3759 | 90.2% |
| Real-snapshot guard | standard | 9 | 8 | 3509 | 38561 | baseline |
| Pointer fast-path | codemap-first | 13 | 7 | 822 | 14210 | 82.1% |
| Pointer fast-path | standard | 14 | 13 | 4775 | 79403 | baseline |

Short version: codemap does not mainly reduce command count. It reduces how much code the agent must read before it can safely edit.

Each benchmark below is recorded separately. The standard path was intentionally reasonable and focused, not a deliberately bad baseline.

### Methods

Each task is executed twice:

```text
A. codemap-first flow
B. rollback
C. standard flow
D. compare metrics
```

Use mixed ordering to reduce memory bias:

| Task | Size | First run | Second run |
|---|---:|---|---|
| Task 1 | 3 steps | codemap-first | standard |
| Task 2 | 5 steps | standard | codemap-first |
| Task 3 | 10 steps | codemap-first | standard |

Between runs, reset the working tree and context:

```powershell
git diff --stat
git diff > .tmp-task-result.patch
git reset --hard
git clean -fd
```

For a fair standard path, use the best conventional workflow you would normally use: focused `rg`, narrow `Get-Content`, targeted `git diff`, and no deliberately wasteful full-repo reads. For a fair codemap path, use `change-plan`, `trace`, `open-set --why`, `signature`, `slice`, `impact`, and `verify-plan` before falling back to raw tools.

### Metrics

Record these metrics for each run:

| Metric | Meaning |
|---|---|
| `commands_count` | Number of terminal commands used for research, edits, and verify |
| `files_opened` | Number of files read through `Get-Content`, `slice`, or full file opens |
| `lines_read` | Approximate lines of source/output consumed as context |
| `terminal_chars` | Characters produced by research commands |
| `estimated_tokens` | `(terminal_chars + read_chars) / 4` |
| `edit_attempts` | Number of patch attempts/fixes |
| `verify_commands` | Number of build/test/check commands |
| `result` | `pass` or `fail` |
| `notes` | Where codemap helped or got in the way |

Measurement helper:

```powershell
function Run-Measured($Name, $Command) {
  $out = Invoke-Expression $Command 2>&1 | Out-String
  [PSCustomObject]@{
    Name = $Name
    Chars = $out.Length
    ApproxTokens = [math]::Ceiling($out.Length / 4)
    Command = $Command
  }
}
```

### Task 1: Scene Snapshot Diagnostic Label Passthrough

Size: small, 3 implementation steps.

Goal: pass a short `diagnostic_label` from scene snapshot service through editor snapshot DTOs so frontend can display whether the scene snapshot is engine, fallback, or cached.

Codemap-first:

```powershell
$cm = "target\debug\amigo-codemap.exe"

Run-Measured "change-plan" "$cm change-plan diagnostic_label --limit 20"
Run-Measured "trace" "$cm trace diagnostic_label --limit 20"
Run-Measured "open-set" "$cm open-set diagnostic_label --why --limit 8"
Run-Measured "signature" "$cm signature SceneSnapshotImage"
Run-Measured "impact" "$cm impact diagnostic_label --limit 20"

& $cm slice crates/tools/scene-snapshot/src/model.rs --symbol SceneSnapshotImage
& $cm slice crates/apps/amigo-editor/src-tauri/src/editor_mode/dto.rs --symbol EditorSceneSnapshotDto
```

Standard:

```powershell
rg "diagnostic_label|SceneSnapshotImage|EditorSceneSnapshotDto"
Get-Content crates/tools/scene-snapshot/src/model.rs
Get-Content crates/tools/scene-snapshot/src/runtime.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/dto.rs
Get-Content crates/apps/amigo-editor/src/api/dto.ts
```

Expected savings: 35-55% fewer context tokens.

Actual discovery benchmark:

| Method | Commands | Files opened | Lines read | Terminal chars | Est. tokens | Result |
|---|---:|---:|---:|---:|---:|---|
| codemap-first | 7 | 2 | 25 | 3764 | 941 | pass |
| standard | 5 | 4 | 2159 | 63495 | 15874 | pass |

Result: codemap-first used ~94.1% fewer estimated context tokens. It avoided opening full DTO/source files and read only the two symbol slices needed for first-pass implementation.

### Task 2: Scene Editor Real-Snapshot Guard

Size: medium, 5 implementation steps.

Goal: make the scene editor clearly distinguish real engine snapshots from fallback snapshots. Picking and drag should be disabled unless the model came from a real engine layout.

Codemap-first:

```powershell
$cm = "target\debug\amigo-codemap.exe"

Run-Measured "change-plan" "$cm change-plan layoutSource --limit 20"
Run-Measured "trace" "$cm trace layoutSource --limit 20"
Run-Measured "open-set" "$cm open-set layoutSource --why --limit 10"
Run-Measured "where" "$cm where EditorSceneSnapshotDto --limit 10"
Run-Measured "impact" "$cm impact layoutSource --limit 30"

& $cm slice crates/apps/amigo-editor/src/api/dto.ts --symbol EditorSceneSnapshotDto
& $cm slice crates/apps/amigo-editor/src/features/scenes/editor/sceneEditorModel.ts --symbol buildSceneEditorModel
& $cm slice crates/apps/amigo-editor/src/features/scenes/editor/SceneEditorCanvas.tsx --symbol SceneEditorCanvas
& $cm symbols --file crates/apps/amigo-editor/src-tauri/src/editor_mode/snapshot.rs --metadata --limit 10
& $cm slice crates/apps/amigo-editor/src-tauri/src/editor_mode/snapshot.rs --symbol fallback_editor_snapshot
```

Standard:

```powershell
rg "layoutSource|EditorSceneSnapshotDto|SceneEditorCanvas|fallback|bounds"
Get-Content crates/apps/amigo-editor/src/api/dto.ts
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/sceneEditorTypes.ts
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/sceneEditorModel.ts
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/SceneEditorCanvas.tsx
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/SceneEditorHud.tsx
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/dto.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/snapshot.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/commands/editor_mode.rs
```

Expected savings: 40-60% fewer context tokens.

Actual discovery benchmark:

| Method | Commands | Files opened | Lines read | Terminal chars | Est. tokens | Result |
|---|---:|---:|---:|---:|---:|---|
| codemap-first | 10 | 5 | 230 | 15034 | 3759 | pass |
| standard | 9 | 8 | 3509 | 154241 | 38561 | pass |

Result: codemap-first used ~90.2% fewer estimated context tokens. `symbols --file --metadata` also corrected the planned backend symbol from a stale guess to the actual `fallback_editor_snapshot` symbol.

### Task 3: Editor Pointer Fast-Path

Size: large, 10 implementation steps.

Goal: reduce `pointerMove` cost in editor mode by adding a lightweight hover/cursor path that does not force full frame image refresh on every move. Full render should remain for down/up/drag/commit or throttled refresh.

Codemap-first:

```powershell
$cm = "target\debug\amigo-codemap.exe"

Run-Measured "change-plan" "$cm change-plan editor pointer fast path --limit 30"
Run-Measured "trace-pointer" "$cm trace SendEditorPointerEvent --limit 30"
Run-Measured "trace-frame" "$cm trace EditorFrameResultDto --limit 30"
Run-Measured "open-set" "$cm open-set pointerMove --why --limit 12"
Run-Measured "tauri-graph" "$cm tauri-graph --limit 80"
Run-Measured "impact" "$cm impact sendEditorPointerEvent --limit 40"

& $cm slice crates/apps/amigo-editor/src/main-window/hooks/useEditorModeCommands.ts --symbol useEditorModeCommands
& $cm slice crates/apps/amigo-editor/src/features/scenes/editor/useSceneEditorPointerEvents.ts --symbol useSceneEditorPointerEvents
& $cm slice crates/apps/amigo-editor/src/features/scenes/editor/SceneEditorCanvas.tsx --symbol SceneEditorCanvas
& $cm symbols --file crates/apps/amigo-editor/src-tauri/src/commands/editor_mode.rs --metadata --limit 12
& $cm symbols --file crates/apps/amigo-editor/src-tauri/src/editor_mode/input.rs --metadata --limit 18
& $cm slice crates/apps/amigo-editor/src-tauri/src/editor_mode/input.rs --symbol handle_pointer_move
& $cm slice crates/apps/amigo-editor/src-tauri/src/editor_mode/session.rs --symbol EditorModeSession
```

Standard:

```powershell
rg "pointerMove|sendEditorPointerEvent|send_editor_pointer_event|EditorFrameResultDto|EditorModeSession|gizmo|hover|cursor"
Get-Content crates/apps/amigo-editor/src/api/dto.ts
Get-Content crates/apps/amigo-editor/src/api/editorApi.ts
Get-Content crates/apps/amigo-editor/src/main-window/hooks/useEditorModeCommands.ts
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/useSceneEditorPointerEvents.ts
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/SceneEditorCanvas.tsx
Get-Content crates/apps/amigo-editor/src/features/scenes/editor/SceneEditorHud.tsx
Get-Content crates/apps/amigo-editor/src-tauri/src/commands/editor_mode.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/dto.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/input.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/session.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/renderer.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/gizmos.rs
Get-Content crates/apps/amigo-editor/src-tauri/src/editor_mode/snapshot.rs
```

Expected savings: 45-65% fewer context tokens.

Actual discovery benchmark:

| Method | Commands | Files opened | Lines read | Terminal chars | Est. tokens | Result |
|---|---:|---:|---:|---:|---:|---|
| codemap-first | 13 | 7 | 822 | 56838 | 14210 | pass |
| standard | 14 | 13 | 4775 | 317610 | 79403 | pass |

Result: codemap-first used ~82.1% fewer estimated context tokens. The main gain came from reading symbol metadata and narrow slices instead of full Tauri/editor-mode files.

### Result Template

```md
## Codemap Token Benchmark - Task 1

### Task
Scene snapshot diagnostic label passthrough

### Method
codemap-first

### Metrics
| Metric | Value |
|---|---:|
| commands_count | 9 |
| files_opened | 3 |
| lines_read | 140 |
| terminal_chars | 9200 |
| estimated_tokens | 2300 |
| edit_attempts | 1 |
| verify_commands | 2 |
| result | pass |

### Notes
Codemap found snapshot/model/DTO path without full repo search.
```

Final comparison table:

| Task | Steps | Method | Files opened | Lines read | Est. tokens | Commands | Result |
|---|---:|---|---:|---:|---:|---:|---|
| Snapshot label | 3 | codemap | 2 | 25 | 941 | 7 | pass |
| Snapshot label | 3 | standard | 4 | 2159 | 15874 | 5 | pass |
| Fallback guard | 5 | codemap | 5 | 230 | 3759 | 10 | pass |
| Fallback guard | 5 | standard | 8 | 3509 | 38561 | 9 | pass |
| Pointer fast-path | 10 | codemap | 7 | 822 | 14210 | 13 | pass |
| Pointer fast-path | 10 | standard | 13 | 4775 | 79403 | 14 | pass |

## Minimal 0.1 Release Smoke Test

Run this before calling the tool usable:

```powershell
cargo fmt -p amigo-codemap --check
cargo test -p amigo-codemap --no-run
cargo test -p amigo-codemap
cargo build -p amigo-codemap

$cm = "target\debug\amigo-codemap.exe"

& $cm brief
& $cm changes --compact --hide-generated --limit 20
& $cm trace patch-apply --limit 20
& $cm open-set patch-apply --why --limit 10
& $cm impact patch-apply --limit 30
& $cm verify-plan --changed
```

Optional ops smoke test:

```powershell
Set-Content -Encoding UTF8 .\tmp.txt "a`nb`nc`n"

$plan = @"
ops:
  - kind: replace_range
    path: tmp.txt
    start_line: 2
    end_line: 2
    content: |
      B
"@
Set-Content -Encoding UTF8 .\ops-test.yml $plan

& $cm ops-preview --from .\ops-test.yml
& $cm ops-check --from .\ops-test.yml
& $cm ops-apply --from .\ops-test.yml --write
Get-Content .\tmp.txt
```

Expected:

```text
a
B
c
```

## Short Command Cheat Sheet

```powershell
# Overview
& $cm brief
& $cm changes --compact --hide-generated

# Files and symbols
& $cm files --query layer:app,kind:source
& $cm symbols --query name:CodeMap
& $cm symbols --file <file> --metadata
& $cm where CodeMap
& $cm signature CodeMap

# String/id tracing
& $cm trace entity.inspector
& $cm trace send_editor_pointer_event

# Reading less code
& $cm open-set entity.inspector --why
& $cm slice <file> --symbol <symbol>

# Planning and impact
& $cm change-plan ui-document
& $cm impact SelectionProperties
& $cm verify-plan --changed

# Safe edits
& $cm patch-check --from patch.diff
& $cm patch-apply --from patch.diff --write
& $cm ops-skeleton <query> --out plan.yml --write
& $cm range-for-symbol <symbol>
& $cm ops-check --from plan.yml
& $cm ops-apply --from plan.yml --write

# Navigation helpers
& $cm explain-file <path>
& $cm neighbors <path>
& $cm api-surface
& $cm component-graph
& $cm tauri-graph
& $cm callsite-candidates <symbol>

# Quality
& $cm todo-index
& $cm risk-index
& $cm smells --top 30 --why
& $cm smells --report --file-lines 500 --min-score 30 --why
```

## Practical Rules

1. Use `trace` for anything ambiguous.
2. Use `where` for known symbols.
3. Use `signature` before opening source.
4. Use `open-set --why` before choosing files.
5. Use `slice --symbol` before reading large files.
6. Use `impact` before editing shared/public code.
7. Use `ops-check` or `patch-check` before writing.
8. Use `verify-plan --changed` after writing.
9. Keep compiler/tests as final truth.
10. Treat experimental graph/symbol-aware operations as helpers, not guarantees.
