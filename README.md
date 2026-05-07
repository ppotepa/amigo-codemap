# amigo-codemap 0.1 Documentation

`amigo-codemap` is the operational navigation and planning layer for the Amigo repository. It helps humans and LLM agents answer "where is this?", "what does this affect?", "what should I read?", and "how can I safely apply this change?" before opening large files or running broad text searches.

It does not replace the compiler, tests, or code review. It replaces a large part of the repetitive discovery work usually done with `rg`, manual file browsing, and ad-hoc patch planning.

## Quickstart

Build the tool:

```powershell
cargo build -p amigo-codemap
```

Optionally create a short alias for the current PowerShell session:

```powershell
$cm = "target\debug\amigo-codemap.exe"
```

Run the default navigation loop:

```powershell
& $cm brief
& $cm changed --group package --limit 20
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
| I want to apply changes safely | `patch-check` / `ops-check` | `& $cm ops-check --from plan.yml` | Validates before write |
| I want to save scope | `workset` | `& $cm workset ui-doc --from-impact UiDocumentEditor --save` | Saved task context |

## Command Reference

### `brief`

Show a compact overview of the repository snapshot.

```powershell
& $cm brief
```

Use when starting work, checking whether codemap sees the repo, or orienting a new agent.

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

### `changed`

Show changed files using git state.

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
```

Always run:

```powershell
& $cm ops-preview --from plan.yml
& $cm ops-check --from plan.yml
& $cm ops-apply --from plan.yml --write
```

Stable operations for 0.1:

```text
create_file
replace_file
replace_range
delete_range
insert_after_anchor
delete_file
```

Symbol-aware operations are implemented but should be treated as experimental until smoke-tested on the target file:

```text
replace_symbol
delete_symbol
insert_before_symbol
insert_after_symbol
replace_method_body
```

### Example: Replace A Line Range

```yaml
version: 1
ops:
  - kind: replace_range
    path: tmp.txt
    start_line: 2
    end_line: 2
    content: |
      B
```

### Example: Create A File

```yaml
version: 1
ops:
  - kind: create_file
    path: crates/apps/amigo-editor/src/example/NewPanel.tsx
    content: |
      export function NewPanel() {
        return <section>New Panel</section>;
      }
```

### Example: Insert After Anchor

```yaml
version: 1
ops:
  - kind: insert_after_anchor
    path: crates/apps/amigo-editor/src/properties/propertiesRegistry.tsx
    anchor: "// @codemap anchor:properties-registry domain:properties role:registry"
    content: |
      import { UiDocumentPropertiesPanel } from "./panels/UiDocumentPropertiesPanel";
```

### Example: Replace Symbol, Experimental

```yaml
version: 1
ops:
  - kind: replace_symbol
    path: crates/tools/amigo-codemap/src/scan/symbols.rs
    symbol: scan_symbols
    content: |
      pub fn scan_symbols(...) -> Result<Vec<SymbolEntry>> {
          todo!("new implementation")
      }
```

Before symbol-aware ops, run:

```powershell
& $cm signature scan_symbols
& $cm slice crates/tools/amigo-codemap/src/scan/symbols.rs --symbol scan_symbols
& $cm ops-check --from plan.yml
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

## Minimal 0.1 Release Smoke Test

Run this before calling the tool usable:

```powershell
cargo fmt -p amigo-codemap --check
cargo test -p amigo-codemap --no-run
cargo test -p amigo-codemap
cargo build -p amigo-codemap

$cm = "target\debug\amigo-codemap.exe"

& $cm brief
& $cm changed --group package --limit 20
& $cm trace patch-apply --limit 20
& $cm open-set patch-apply --why --limit 10
& $cm impact patch-apply --limit 30
& $cm verify-plan --changed
```

Optional ops smoke test:

```powershell
Set-Content -Encoding UTF8 .\tmp.txt "a`nb`nc`n"

$plan = @"
version: 1
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
& $cm changed --group package

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
