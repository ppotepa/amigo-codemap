# amigo-codemap

Workspace code map generator for LLM-assisted development.

## Responsibility
- Build a compact, language-agnostic index of workspace structure.
- Use existing project metadata such as Cargo and package manifests.
- Emit compact summaries for implementation planning and navigation.
- Provide small task-focused reports before an LLM reads file contents.

## Not here
- Engine or editor domain knowledge.
- Rust or TypeScript semantic analysis owned by this project.
- Runtime code generation.

## Depends on
- cargo metadata.
- serde.
- serde_json.

## Commands
- `brief` - tiny repo summary.
- `compact` - compact JSON written to `.amigo/codemap.json`.
- `changed --group path|package|language|status` - grouped dirty worktree summary.
- `find <text>` - literal search across indexed text files.
- `scope <query>` - small context for a file, area, package, or symbol.
- `refs <query>` - definitions plus text references, including CSS selectors at level 2.
- `docs` - README coverage for workspace packages.
- `command-map <name>` - points to CLI, dispatch, implementation, docs, and tests for one codemap command.
- `verify <profile>` - capped command output for `npm-build`, `npm-test`, `cargo-check`, or `cargo-test`.

## Refactor reports

These commands provide compact operational context for LLM-assisted refactors.

- `verify-plan --changed` - suggests the smallest useful verification commands.
- `stale --patterns a,b,c` - finds stale aliases, placeholders, old names, and cleanup candidates.
- `impact <symbol> --group feature` - groups direct refs and likely affected areas.
- `fallout [--from file]` - summarizes TypeScript/Rust build output.
- `move-plan <file> --by tauri-command|symbol` - suggests split groups and move risks.
- `dup [symbol]` - finds duplicate helpers by symbol name and simple normalized bodies.
- `tauri-commands` - checks command definitions against `generate_handler!`.
- `service-shape <TypeName>` - groups service bag fields by usage.
- `registry-check [kind]` - checks known editor registries.
- `operations-summary` - summarizes costly tasks from `operations.md`.
- `commit-summary --changed` - creates a compact change summary.
- `append-plan <file> [--task name]` - suggests append anchors, donor files, and companion files for additive edits.
- `copy-plan <target> [--from donor] [--task name]` - picks a donor file, rename hotspots, and mirrored companion files for copy-driven edits.
- `slice <file> --symbol <name> [--radius N]` - compact file fragment around one symbol.
- `diff-scope` - changed files summary by symbols and import-level risk.
- `delete-plan <file> [--changed]` - checks whether file can be removed safely.
- `file-move-plan <from> --to <to>` - estimates import fallout and inbound imports.
- `rename-plan <old> --to <new> [--group feature]` - exact vs partial rename hits.
- `import-fix-plan [--changed]` - finds missing/stale relative imports.
- `open-set <symbol> [--task migrate]` - proposes best file-read order and skips low-value docs/fixtures.
- `workset <name> [--from-impact symbol] [--save|--status]` - manage long refactor context (manifest).
- `barrel-check <dir>` - checks export barrels and duplicates.
- `orphan-files <dir>` - finds files without inbound usage.
- `shim-check [--changed]` - flags tiny files that are probably shims.
- `large-files [--top N] [--with-split-hints]` - ranking for future split candidates.
- `asset-file-check <mod>` - checks YAML asset ids and source references.
- `case-check [--changed]` - catches case-sensitive import collisions.
- `text-check [--changed]` - line endings/BOM/binary/text quick pass.
- `patch-preview --from patch.diff` - summaries changed symbols and risk before apply.
- `commit-files [--changed]` - suggests logical commit bundles.

## Examples
```powershell
cargo run -p amigo-codemap -- brief
cargo run -p amigo-codemap -- changed --group package --limit 20
cargo run -p amigo-codemap -- find "AssetTreePanel" --limit 20
cargo run -p amigo-codemap -- scope AssetTreePanel --limit 30
cargo run -p amigo-codemap -- refs asset-tree-section --limit 20
cargo run -p amigo-codemap -- docs
cargo run -p amigo-codemap -- command-map append-plan
cargo run -p amigo-codemap -- command-map copy-plan
cargo run -p amigo-codemap -- verify-plan --changed
cargo run -p amigo-codemap -- impact EditorSelectionRef --group feature --limit 80
cargo run -p amigo-codemap -- stale --patterns workspacePanels,createEditorSelection
cargo run -p amigo-codemap -- move-plan crates/apps/amigo-editor/src-tauri/src/commands/mod.rs --by tauri-command
npm run build 2>&1 | cargo run -p amigo-codemap -- fallout --limit 80
cargo run -p amigo-codemap -- tauri-commands
cargo run -p amigo-codemap -- diff-scope --changed --limit 80
cargo run -p amigo-codemap -- open-set EditorSelectionRef --task migrate --limit 12
cargo run -p amigo-codemap -- append-plan crates/apps/amigo-editor/src/editor-components/builtinComponents.tsx --task component-definition --limit 12
cargo run -p amigo-codemap -- copy-plan crates/apps/amigo-editor/src/startup/NewPanel.tsx --from crates/apps/amigo-editor/src/startup/ModsPanel.tsx --task panel --limit 12
cargo run -p amigo-codemap -- file-move-plan crates/apps/amigo-editor/src/assets/AssetTreePanel.tsx --to crates/apps/amigo-editor/src/features/assets/AssetTreePanel.tsx
cargo run -p amigo-codemap -- workset selection-migration --from-impact EditorSelectionRef --save
cargo run -p amigo-codemap -- workset selection-migration --status
cargo run -p amigo-codemap -- large-files --top 20 --with-split-hints
cargo run -p amigo-codemap -- stale --patterns workspacePanels,createEditorSelection --limit 80
cargo run -p amigo-codemap -- delete-plan crates/apps/amigo-editor/src/main-window/workspacePanels.tsx
cargo run -p amigo-codemap -- import-fix-plan --changed
cargo run -p amigo-codemap -- patch-preview --from patch.diff
cargo run -p amigo-codemap -- commit-files --changed
cargo run -p amigo-codemap -- commit-summary --changed
```

## How to use `command-map`, `append-plan`, and `copy-plan`

### `command-map`

Use `command-map <name>` when you are extending `amigo-codemap` itself and do not want to fall back to manual repo search.

```powershell
cargo run -p amigo-codemap -- command-map copy-plan
```

Read the output in this order:
- `cli` - where the command is parsed.
- `dispatch` - where `main.rs` routes it.
- `implementation` - the actual report file.
- `docs` - README/workflow entries to update.
- `tests` - the narrowest tests to extend or run.

Default workflow:
1. run `command-map <name>`
2. read `cli`, then `dispatch`, then `implementation`
3. edit code
4. update docs from the `docs` list
5. run the targeted tests from the `tests` list

### `append-plan`

Use `append-plan <file> --task ...` when the target file already exists and you want to add a new block, registry entry, route, style rule, or test case.

```powershell
cargo run -p amigo-codemap -- append-plan crates/apps/amigo-editor/src/editor-components/builtinComponents.tsx --task component-definition --limit 12
```

How to read it:
- `append anchors` - preferred insert points; use the first structural anchor instead of blind EOF append.
- `symbol context` - nearby top-level declarations in the file.
- `donor candidates` - similar files to borrow a small pattern from.
- `companion files` - likely follow-up files for imports, registration, or styles.

Default workflow:
1. run `append-plan`
2. pick the first structural anchor
3. read one donor candidate only if the change is mechanical
4. check companion files before saving
5. run the suggested verify commands

### `copy-plan`

Use `copy-plan <target> [--from donor] [--task ...]` when you want to create a new file from an existing pattern or transplant a larger block from a known donor file.

```powershell
cargo run -p amigo-codemap -- copy-plan crates/apps/amigo-editor/src/startup/NewPanel.tsx --from crates/apps/amigo-editor/src/startup/ModsPanel.tsx --task panel --limit 12
```

How to read it:
- `selected donor` - the file to copy from; if you do not pass `--from`, the report ranks one for you.
- `alternate donors` - backup options; usually you should not read more than the top 1-2.
- `rename hotspots` - names, symbols, and relative imports to fix first.
- `mirrored companion files` - CSS/tests/helpers that may need a sibling copy.
- `target anchors` - where to insert the copied block if the target file already exists.

Default workflow:
1. run `copy-plan`
2. accept the top donor unless you have a clear reason not to
3. rename hotspots before cleaning imports and props
4. if the target already exists, run `append-plan <target>` before inserting the copied block
5. add mirrored companion files only if the donor really depends on them

## Problem to command

| Problem | First command | Follow-up |
| --- | --- | --- |
| What changed? | `changed --group package` | `diff-scope --changed` |
| What should I verify? | `verify-plan --changed` | `fallout --from ...` |
| Where is this codemap command wired? | `command-map <name>` | `scope` on the reported files |
| What files should I read first? | `open-set <symbol> --task migrate` | `slice <file> --symbol <name>` |
| Where should I append a new block or registry entry? | `append-plan <file> --task ...` | `open-set <symbol>` |
| What donor file should I copy and what must I rename? | `copy-plan <target> --task ...` | `append-plan <target>` |
| What does a symbol change affect? | `impact <symbol> --group feature` | `workset <name> --from-impact <symbol> --save` |
| Can I delete this file? | `delete-plan <file>` | `stale --patterns ...` |
| What breaks if I move this file? | `file-move-plan <from> --to <to>` | `import-fix-plan --changed` |
| Which imports are stale or missing? | `import-fix-plan --changed` | `npm run build` + `fallout` |
| Which files are probably dead shims? | `orphan-files <dir>` | `shim-check --changed` |
| Which big file should I split next? | `large-files --top 20 --with-split-hints` | `move-plan <file>` |
| How should I split the work into commits? | `commit-files --changed` | `commit-summary --changed` |

## Manual test scenarios

Repository root also contains `codemap-tests/` with three multi-step scenarios:
- `001-symbol-migration`
- `002-file-ops-cleanup`
- `003-large-file-split`

Each scenario includes:
- `task.md` with the command sequence,
- `result.md` for token and workflow notes,
- shared rollup in `codemap-tests/summary.md`.
