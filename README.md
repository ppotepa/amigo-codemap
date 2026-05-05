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
cargo run -p amigo-codemap -- verify-plan --changed
cargo run -p amigo-codemap -- impact EditorSelectionRef --group feature --limit 80
cargo run -p amigo-codemap -- stale --patterns workspacePanels,createEditorSelection
cargo run -p amigo-codemap -- move-plan crates/apps/amigo-editor/src-tauri/src/commands/mod.rs --by tauri-command
npm run build 2>&1 | cargo run -p amigo-codemap -- fallout --limit 80
cargo run -p amigo-codemap -- tauri-commands
cargo run -p amigo-codemap -- diff-scope --changed --limit 80
cargo run -p amigo-codemap -- open-set EditorSelectionRef --task migrate --limit 12
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
