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

## Examples
```powershell
cargo run -p amigo-codemap -- brief
cargo run -p amigo-codemap -- changed --group package --limit 20
cargo run -p amigo-codemap -- find "AssetTreePanel" --limit 20
cargo run -p amigo-codemap -- scope AssetTreePanel --limit 30
cargo run -p amigo-codemap -- refs asset-tree-section --limit 20
cargo run -p amigo-codemap -- docs
```
