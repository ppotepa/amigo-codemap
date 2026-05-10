use std::path::PathBuf;

use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub struct Options {
    pub root: PathBuf,
    pub out: PathBuf,
    pub level: u8,
    pub pretty: bool,
    pub ai: bool,
    pub query: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub group: Option<String>,
    pub lines: bool,
    pub line_range: Option<String>,
    pub limit: usize,
    pub verify_args: Vec<String>,
    pub changed_only: bool,
    pub patterns: Vec<String>,
    pub file: Option<PathBuf>,
    pub from: Option<PathBuf>,
    pub yaml: Option<String>,
    pub yaml_op: String,
    pub by: Option<String>,
    pub to: Option<PathBuf>,
    pub symbol: Option<String>,
    pub task: Option<String>,
    pub from_impact: Option<String>,
    pub radius: usize,
    pub context_radius: usize,
    pub top: usize,
    pub with_split_hints: bool,
    pub save: bool,
    pub status: bool,
    pub write: bool,
    pub strict: bool,
    pub backup: bool,
    pub stop_on_error: bool,
    pub run: bool,
    pub why: bool,
    pub metadata: bool,
    pub json: bool,
    pub raw: bool,
    pub no_verbose: bool,
    pub quiet: bool,
    pub no_cache: bool,
    pub compact: bool,
    pub hide_generated: bool,
    pub warnings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Scan,
    Refresh,
    Watch,
    Status,
    Changes,
    Files,
    Changed,
    Symbols,
    Where,
    Signature,
    Trace,
    ChangePlan,
    ExplainFile,
    Neighbors,
    ApiSurface,
    ComponentGraph,
    TauriGraph,
    CallsiteCandidates,
    TodoIndex,
    RiskIndex,
    Compact,
    Explain,
    Brief,
    Find,
    Scope,
    Refs,
    Docs,
    CommandMap,
    Anchors,
    AnchorCheck,
    Taxonomy,
    Verify,
    VerifyPlan,
    Stale,
    Impact,
    Fallout,
    MovePlan,
    Dup,
    TauriCommands,
    ServiceShape,
    RegistryCheck,
    MetadataAudit,
    DescriptorSkeleton,
    OperationsSummary,
    CommitPlan,
    CommitSummary,
    AppendPlan,
    CopyPlan,
    Slice,
    DiffScope,
    DeletePlan,
    FileMovePlan,
    RenamePlan,
    ImportFixPlan,
    OpenSet,
    Workset,
    BarrelCheck,
    OrphanFiles,
    ShimCheck,
    LargeFiles,
    AssetFileCheck,
    CaseCheck,
    TextCheck,
    PatchPreview,
    PatchCheck,
    PatchApply,
    OpsPreview,
    OpsCheck,
    OpsApply,
    OpsRawPreview,
    OpsRawCheck,
    OpsRawApply,
    OpsSkeleton,
    OpsSchema,
    OpsSplit,
    OpsVerify,
    OpsSummary,
    RangeForSymbol,
    RangeForLines,
    AnchorRange,
    CommitFiles,
}

#[derive(Debug, Clone)]
pub struct Cli {
    pub command: Command,
    pub options: Options,
}

impl Cli {
    pub fn parse<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
    {
        let mut command = None;
        let mut root = std::env::current_dir()?;
        let mut out = None;
        let mut level = 1;
        let mut pretty = false;
        let mut ai = false;
        let mut query = None;
        let mut start_line = None;
        let mut end_line = None;
        let mut group = None;
        let mut lines = false;
        let mut line_range = None;
        let mut limit = 80;
        let mut verify_args = Vec::new();
        let mut changed_only = false;
        let mut patterns = Vec::new();
        let mut file = None;
        let mut from = None;
        let mut yaml = None;
        let mut yaml_op = "replace_range".to_string();
        let mut by = None;
        let mut to = None;
        let mut symbol = None;
        let mut task = None;
        let mut from_impact = None;
        let mut radius = 32usize;
        let mut context_radius = 3usize;
        let mut top = 20usize;
        let mut with_split_hints = false;
        let mut save = false;
        let mut status = false;
        let mut write = false;
        let mut strict = false;
        let mut backup = false;
        let mut stop_on_error = false;
        let mut run = false;
        let mut why = false;
        let mut metadata = false;
        let mut json = false;
        let mut raw = false;
        let mut no_verbose = false;
        let mut quiet = false;
        let mut no_cache = false;
        let mut compact = false;
        let mut hide_generated = false;
        let mut warnings = false;

        let args = args.into_iter().collect::<Vec<_>>();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if command.is_some() && !arg.starts_with('-') && parse_command_name(arg).is_some() {
                match command {
                    Some(
                        Command::Find
                        | Command::Symbols
                        | Command::Where
                        | Command::Signature
                        | Command::Trace
                        | Command::ChangePlan
                        | Command::ExplainFile
                        | Command::Neighbors
                        | Command::CallsiteCandidates
                        | Command::Scope
                        | Command::Refs
                        | Command::Docs
                        | Command::CommandMap
                        | Command::Anchors
                        | Command::Impact
                        | Command::MovePlan
                        | Command::Dup
                        | Command::ServiceShape
                        | Command::RegistryCheck
                        | Command::MetadataAudit
                        | Command::DescriptorSkeleton
                        | Command::AppendPlan
                        | Command::CopyPlan
                        | Command::Slice
                        | Command::DeletePlan
                        | Command::FileMovePlan
                        | Command::RenamePlan
                        | Command::OpenSet
                        | Command::Workset
                        | Command::BarrelCheck
                        | Command::OrphanFiles
                        | Command::AssetFileCheck
                        | Command::PatchPreview
                        | Command::PatchCheck
                        | Command::PatchApply
                        | Command::OpsPreview
                        | Command::OpsCheck
                        | Command::OpsApply
                        | Command::OpsSkeleton
                        | Command::OpsSchema
                        | Command::OpsSplit
                        | Command::OpsVerify
                        | Command::OpsSummary
                        | Command::RangeForSymbol
                        | Command::RangeForLines
                        | Command::AnchorRange
                        | Command::CommitFiles,
                    ) if query.is_none() => {
                        query = Some(arg.to_owned());
                        index += 1;
                        continue;
                    }
                    Some(Command::Verify) => {
                        verify_args.push(arg.to_owned());
                        index += 1;
                        continue;
                    }
                    _ => bail!("unexpected positional `{arg}`"),
                }
            }
            match arg.as_str() {
                "scan" => command = Some(Command::Scan),
                "refresh" => command = Some(Command::Refresh),
                "watch" => command = Some(Command::Watch),
                "status" => command = Some(Command::Status),
                "changes" => command = Some(Command::Changes),
                "files" => command = Some(Command::Files),
                "changed" => command = Some(Command::Changed),
                "symbols" => command = Some(Command::Symbols),
                "where" => command = Some(Command::Where),
                "signature" => command = Some(Command::Signature),
                "trace" => command = Some(Command::Trace),
                "change-plan" => command = Some(Command::ChangePlan),
                "explain-file" => command = Some(Command::ExplainFile),
                "neighbors" => command = Some(Command::Neighbors),
                "api-surface" => command = Some(Command::ApiSurface),
                "component-graph" => command = Some(Command::ComponentGraph),
                "tauri-graph" => command = Some(Command::TauriGraph),
                "callsite-candidates" => command = Some(Command::CallsiteCandidates),
                "todo-index" => command = Some(Command::TodoIndex),
                "risk-index" => command = Some(Command::RiskIndex),
                "compact" => command = Some(Command::Compact),
                "brief" => command = Some(Command::Brief),
                "find" => command = Some(Command::Find),
                "scope" => command = Some(Command::Scope),
                "refs" => command = Some(Command::Refs),
                "docs" | "readme-coverage" => command = Some(Command::Docs),
                "command-map" => command = Some(Command::CommandMap),
                "anchors" => command = Some(Command::Anchors),
                "anchor-check" => command = Some(Command::AnchorCheck),
                "taxonomy" => command = Some(Command::Taxonomy),
                "verify" => command = Some(Command::Verify),
                "verify-plan" => command = Some(Command::VerifyPlan),
                "stale" => command = Some(Command::Stale),
                "impact" => command = Some(Command::Impact),
                "fallout" => command = Some(Command::Fallout),
                "move-plan" => command = Some(Command::MovePlan),
                "dup" => command = Some(Command::Dup),
                "tauri-commands" => command = Some(Command::TauriCommands),
                "service-shape" => command = Some(Command::ServiceShape),
                "registry-check" => command = Some(Command::RegistryCheck),
                "metadata-audit" => command = Some(Command::MetadataAudit),
                "descriptor-skeleton" => command = Some(Command::DescriptorSkeleton),
                "operations-summary" => command = Some(Command::OperationsSummary),
                "commit-plan" => command = Some(Command::CommitPlan),
                "commit-summary" => command = Some(Command::CommitSummary),
                "append-plan" => command = Some(Command::AppendPlan),
                "copy-plan" => command = Some(Command::CopyPlan),
                "slice" => command = Some(Command::Slice),
                "diff-scope" => command = Some(Command::DiffScope),
                "delete-plan" => command = Some(Command::DeletePlan),
                "file-move-plan" => command = Some(Command::FileMovePlan),
                "rename-plan" => command = Some(Command::RenamePlan),
                "import-fix-plan" => command = Some(Command::ImportFixPlan),
                "open-set" => command = Some(Command::OpenSet),
                "workset" => command = Some(Command::Workset),
                "barrel-check" => command = Some(Command::BarrelCheck),
                "orphan-files" => command = Some(Command::OrphanFiles),
                "shim-check" => command = Some(Command::ShimCheck),
                "large-files" => command = Some(Command::LargeFiles),
                "asset-file-check" => command = Some(Command::AssetFileCheck),
                "case-check" => command = Some(Command::CaseCheck),
                "text-check" => command = Some(Command::TextCheck),
                "patch-preview" => command = Some(Command::PatchPreview),
                "patch-check" => command = Some(Command::PatchCheck),
                "patch-apply" => command = Some(Command::PatchApply),
                "ops-preview" => command = Some(Command::OpsPreview),
                "ops-check" => command = Some(Command::OpsCheck),
                "ops-apply" => command = Some(Command::OpsApply),
                "ops-raw-preview" => command = Some(Command::OpsRawPreview),
                "ops-raw-check" => command = Some(Command::OpsRawCheck),
                "ops-raw-apply" => command = Some(Command::OpsRawApply),
                "ops-skeleton" => command = Some(Command::OpsSkeleton),
                "ops-schema" => command = Some(Command::OpsSchema),
                "ops-split" => command = Some(Command::OpsSplit),
                "ops-verify" => command = Some(Command::OpsVerify),
                "ops-summary" => command = Some(Command::OpsSummary),
                "range-for-symbol" => command = Some(Command::RangeForSymbol),
                "range-for-lines" => command = Some(Command::RangeForLines),
                "anchor-range" => command = Some(Command::AnchorRange),
                "commit-files" => command = Some(Command::CommitFiles),
                "explain" | "--help" | "-h" => command = Some(Command::Explain),
                "--root" => {
                    index += 1;
                    root = PathBuf::from(required_value(&args, index, "--root")?);
                }
                "--out" => {
                    index += 1;
                    out = Some(PathBuf::from(required_value(&args, index, "--out")?));
                }
                "--level" => {
                    index += 1;
                    level = required_value(&args, index, "--level")?.parse::<u8>()?;
                    if level > 3 {
                        bail!("--level must be 0, 1, 2, or 3");
                    }
                }
                "--pretty" => pretty = true,
                "--ai" => ai = true,
                "--group" => {
                    index += 1;
                    group = Some(required_value(&args, index, "--group")?);
                }
                "--query" => {
                    index += 1;
                    query = Some(required_value(&args, index, "--query")?);
                }
                "--example" => {
                    index += 1;
                    query = Some(required_value(&args, index, "--example")?);
                }
                "--lines" => {
                    lines = true;
                    if args.get(index + 1).is_some_and(|value| {
                        !value.starts_with('-') && parse_command_name(value).is_none()
                    }) {
                        index += 1;
                        line_range = Some(required_value(&args, index, "--lines")?);
                    }
                }
                "--changed" => changed_only = true,
                "--patterns" => {
                    index += 1;
                    patterns = required_value(&args, index, "--patterns")?
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                "--file" => {
                    index += 1;
                    file = Some(PathBuf::from(required_value(&args, index, "--file")?));
                }
                "--from" => {
                    index += 1;
                    from = Some(PathBuf::from(required_value(&args, index, "--from")?));
                }
                "--yaml" => {
                    index += 1;
                    yaml = Some(required_value(&args, index, "--yaml")?);
                }
                "--yaml-op" => {
                    index += 1;
                    yaml_op = required_value(&args, index, "--yaml-op")?;
                }
                "--by" => {
                    index += 1;
                    by = Some(required_value(&args, index, "--by")?);
                }
                "--to" => {
                    index += 1;
                    to = Some(PathBuf::from(required_value(&args, index, "--to")?));
                }
                "--symbol" => {
                    index += 1;
                    symbol = Some(required_value(&args, index, "--symbol")?);
                }
                "--task" => {
                    index += 1;
                    task = Some(required_value(&args, index, "--task")?);
                }
                "--from-impact" => {
                    index += 1;
                    from_impact = Some(required_value(&args, index, "--from-impact")?);
                }
                "--radius" => {
                    index += 1;
                    radius = required_value(&args, index, "--radius")?.parse::<usize>()?;
                }
                "--context-radius" => {
                    index += 1;
                    context_radius =
                        required_value(&args, index, "--context-radius")?.parse::<usize>()?;
                }
                "--limit" => {
                    index += 1;
                    limit = required_value(&args, index, "--limit")?.parse::<usize>()?;
                }
                "--top" => {
                    index += 1;
                    top = required_value(&args, index, "--top")?.parse::<usize>()?;
                }
                "--with-split-hints" => with_split_hints = true,
                "--save" => save = true,
                "--status" => status = true,
                "--write" => write = true,
                "--strict" => strict = true,
                "--backup" => backup = true,
                "--stop-on-error" => stop_on_error = true,
                "--run" => run = true,
                "--why" => why = true,
                "--metadata" => metadata = true,
                "--json" => json = true,
                "--raw" => raw = true,
                "--no-verbose" => no_verbose = true,
                "--quiet" => quiet = true,
                "--no-cache" => no_cache = true,
                "--compact" => compact = true,
                "--hide-generated" => hide_generated = true,
                "--warnings" => warnings = true,
                unknown if unknown.starts_with('-') => bail!("unknown flag `{unknown}`"),
                value => match command {
                    Some(
                        Command::Find
                        | Command::Symbols
                        | Command::Where
                        | Command::Signature
                        | Command::Trace
                        | Command::ChangePlan
                        | Command::ExplainFile
                        | Command::Neighbors
                        | Command::CallsiteCandidates
                        | Command::Scope
                        | Command::Refs
                        | Command::Docs
                        | Command::CommandMap
                        | Command::Anchors
                        | Command::Impact
                        | Command::MovePlan
                        | Command::Dup
                        | Command::ServiceShape
                        | Command::RegistryCheck
                        | Command::MetadataAudit
                        | Command::DescriptorSkeleton
                        | Command::Slice
                        | Command::AppendPlan
                        | Command::CopyPlan
                        | Command::DeletePlan
                        | Command::FileMovePlan
                        | Command::RenamePlan
                        | Command::OpenSet
                        | Command::Workset
                        | Command::BarrelCheck
                        | Command::OrphanFiles
                        | Command::AssetFileCheck
                        | Command::PatchPreview
                        | Command::PatchCheck
                        | Command::PatchApply
                        | Command::OpsPreview
                        | Command::OpsCheck
                        | Command::OpsApply
                        | Command::OpsRawPreview
                        | Command::OpsRawCheck
                        | Command::OpsRawApply
                        | Command::OpsSkeleton
                        | Command::OpsSchema
                        | Command::OpsSplit
                        | Command::OpsVerify
                        | Command::OpsSummary
                        | Command::RangeForSymbol
                        | Command::RangeForLines
                        | Command::AnchorRange
                        | Command::CommitFiles,
                    ) if query.is_none() => {
                        query = Some(value.to_owned());
                    }
                    Some(Command::RangeForLines) if start_line.is_none() => {
                        start_line = Some(value.parse::<usize>()?);
                    }
                    Some(Command::RangeForLines) if end_line.is_none() => {
                        end_line = Some(value.parse::<usize>()?);
                    }
                    Some(Command::Verify) => verify_args.push(value.to_owned()),
                    _ => bail!("unknown command `{value}`"),
                },
            }
            index += 1;
        }

        let root = root.canonicalize().unwrap_or(root);
        let out = out.unwrap_or_else(|| root.join(".amigo").join("codemap.json"));

        Ok(Self {
            command: command.unwrap_or(Command::Scan),
            options: Options {
                root,
                out,
                level,
                pretty,
                ai,
                query,
                start_line,
                end_line,
                group,
                lines,
                line_range,
                limit,
                verify_args,
                changed_only,
                patterns,
                file,
                from,
                yaml,
                yaml_op,
                by,
                to,
                symbol,
                task,
                from_impact,
                radius,
                context_radius,
                top,
                with_split_hints,
                save,
                status,
                write,
                strict,
                backup,
                stop_on_error,
                run,
                why,
                metadata,
                json,
                raw,
                no_verbose,
                quiet,
                no_cache,
                compact,
                hide_generated,
                warnings,
            },
        })
    }
}

pub fn print_help() {
    println!(
        "amigo-codemap\n\ncommands:\n  scan\n  refresh           refresh compact output and fast snapshot cache\n  watch\n  status            show fast snapshot cache status\n  changes           live git status + shortstat summary\n  files [--query tag1,tag2] [--group tag|path|language|package] [--changed]\n  symbols [--query ...] [--file path] [--metadata]\n  brief\n  compact\n  changed --group path|package|language|status\n  find <text>\n  scope <query>\n  refs <query>\n  docs\n  command-map <name>\n  taxonomy\n  anchors [query] [--write]\n  anchor-check\n  verify <profile>\n  verify-plan [--changed]\n  stale --patterns a,b,c [--changed]\n  impact <symbol> [--group feature|path|package]\n  fallout [--from file]\n  move-plan <file> [--by tauri-command|symbol]\n  dup [symbol] [--changed]\n  append-plan <file> [--task name]\n  copy-plan <target> [--from donor] [--task name]\n  slice <file> [--symbol Name] [--radius N]\n  diff-scope [--changed]\n  delete-plan <file> [--changed]\n  file-move-plan <from> --to <to>\n  rename-plan <old> --to <new>\n  import-fix-plan [--changed]\n  open-set <query> [--task name]\n  workset <name> [--from-impact symbol] [--save|--status]\n  barrel-check <dir>\n  orphan-files <dir>\n  shim-check [--changed]\n  large-files [--top N] [--with-split-hints]\n  asset-file-check <query>\n  case-check [--changed]\n  text-check [--changed]\n  patch-preview [--from patch.diff]\n  patch-check [--from patch.diff]\n  patch-apply [--from patch.diff] [--write]\n  ops-schema [--json] [--example kind]\n  ops-preview [--from plan.yml|--from -|--yaml text]\n  ops-check [--from plan.yml|--from -|--yaml text] [--strict]\n  ops-apply [--from plan.yml|--from -|--yaml text] [--write] [--backup] [--stop-on-error] [--no-verbose]\n  ops-skeleton <query> [--out plan.yml] [--write]\n  ops-split [--from plan.yml|--yaml text] [--by domain|risk]\n  ops-verify [--from plan.yml|--yaml text] [--run]\n  ops-summary [--from plan.yml|--yaml text] [--changed]\n  range-for-symbol <symbol>\n  range-for-lines <path> <start-line> <end-line> [--yaml-op replace_range|delete_range]\n  anchor-range <anchor> [--to end-anchor]\n  commit-files [--changed]\n  commit-plan        live git grouped commit plan\n  tauri-commands\n  service-shape <TypeName>\n  registry-check [properties|components|file-rules|project-actions]\n  operations-summary\n  commit-summary [--changed]\n\nflags:\n  --root <path>    project root, defaults to cwd\n  --out <path>     output path, defaults to .amigo/codemap.json\n  --level <0-3>    0 files, 1 public/export symbols, 2 local symbols, 3 relations\n  --pretty         pretty JSON\n  --ai             compact/minified JSON\n  --group <kind>   group output by path|package|language|status|feature|tag|domain\n  --lines          include matching lines where supported\n  --changed        focus on git changed files\n  --patterns <a,b> stale patterns\n  --file <path>    focus reports on one file where supported\n  --from <path>    fallout/patch/ops input file; use - for stdin\n  --yaml <text>    inline ops-plan YAML input\n  --yaml-op <kind> range-for-lines op kind: replace_range or delete_range\n  --from-impact <symbol> build workset from impact refs\n  --by <kind>      move/dup/split strategy\n  --to <path>      move target, rename destination, or end anchor\n  --symbol <name>  slice symbol/rename source\n  --task <name>    open-set/workset/append/copy context task\n  --radius <n>     slice context radius\n  --context-radius <n> range-for-lines context radius\n  --top <n>        top-N listing for ranking commands\n  --with-split-hints include split hints in large-files\n  --save           persist workset\n  --status         show workset status\n  --write          allow write-capable commands to modify files\n  --strict         fail unsafe ops-plan locators\n  --backup         create .amigo/ops-backups before ops-apply writes\n  --stop-on-error  stop ops-apply after first failed operation\n  --run            allow command-specific execution mode where supported\n  --json           JSON output where supported\n  --no-verbose     reduce ops-apply output\n  --quiet          suppress timing summary\n  --example <kind> select ops-schema example kind\n  --why            include ranking reasons where supported\n  --metadata       include expanded metadata where supported\n  --compact        compact output for changes/commit-plan\n  --hide-generated hide generated/index files in changes output\n  --warnings       show only live git warnings where supported\n  --no-cache       force full scan instead of reading .amigo/codemap.snapshot.json\n  --limit <n>      output row cap, default 80"
    );
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn parse_command_name(value: &str) -> Option<Command> {
    match value {
        "scan" => Some(Command::Scan),
        "refresh" => Some(Command::Refresh),
        "watch" => Some(Command::Watch),
        "status" => Some(Command::Status),
        "changes" => Some(Command::Changes),
        "files" => Some(Command::Files),
        "changed" => Some(Command::Changed),
        "symbols" => Some(Command::Symbols),
        "where" => Some(Command::Where),
        "signature" => Some(Command::Signature),
        "trace" => Some(Command::Trace),
        "change-plan" => Some(Command::ChangePlan),
        "explain-file" => Some(Command::ExplainFile),
        "neighbors" => Some(Command::Neighbors),
        "api-surface" => Some(Command::ApiSurface),
        "component-graph" => Some(Command::ComponentGraph),
        "tauri-graph" => Some(Command::TauriGraph),
        "callsite-candidates" => Some(Command::CallsiteCandidates),
        "todo-index" => Some(Command::TodoIndex),
        "risk-index" => Some(Command::RiskIndex),
        "compact" => Some(Command::Compact),
        "brief" => Some(Command::Brief),
        "find" => Some(Command::Find),
        "scope" => Some(Command::Scope),
        "refs" => Some(Command::Refs),
        "docs" | "readme-coverage" => Some(Command::Docs),
        "command-map" => Some(Command::CommandMap),
        "anchors" => Some(Command::Anchors),
        "anchor-check" => Some(Command::AnchorCheck),
        "taxonomy" => Some(Command::Taxonomy),
        "verify" => Some(Command::Verify),
        "verify-plan" => Some(Command::VerifyPlan),
        "stale" => Some(Command::Stale),
        "impact" => Some(Command::Impact),
        "fallout" => Some(Command::Fallout),
        "move-plan" => Some(Command::MovePlan),
        "dup" => Some(Command::Dup),
        "tauri-commands" => Some(Command::TauriCommands),
        "service-shape" => Some(Command::ServiceShape),
        "registry-check" => Some(Command::RegistryCheck),
        "metadata-audit" => Some(Command::MetadataAudit),
        "descriptor-skeleton" => Some(Command::DescriptorSkeleton),
        "operations-summary" => Some(Command::OperationsSummary),
        "commit-plan" => Some(Command::CommitPlan),
        "commit-summary" => Some(Command::CommitSummary),
        "append-plan" => Some(Command::AppendPlan),
        "copy-plan" => Some(Command::CopyPlan),
        "slice" => Some(Command::Slice),
        "diff-scope" => Some(Command::DiffScope),
        "delete-plan" => Some(Command::DeletePlan),
        "file-move-plan" => Some(Command::FileMovePlan),
        "rename-plan" => Some(Command::RenamePlan),
        "import-fix-plan" => Some(Command::ImportFixPlan),
        "open-set" => Some(Command::OpenSet),
        "workset" => Some(Command::Workset),
        "barrel-check" => Some(Command::BarrelCheck),
        "orphan-files" => Some(Command::OrphanFiles),
        "shim-check" => Some(Command::ShimCheck),
        "large-files" => Some(Command::LargeFiles),
        "asset-file-check" => Some(Command::AssetFileCheck),
        "case-check" => Some(Command::CaseCheck),
        "text-check" => Some(Command::TextCheck),
        "patch-preview" => Some(Command::PatchPreview),
        "patch-check" => Some(Command::PatchCheck),
        "patch-apply" => Some(Command::PatchApply),
        "ops-preview" => Some(Command::OpsPreview),
        "ops-check" => Some(Command::OpsCheck),
        "ops-apply" => Some(Command::OpsApply),
        "ops-raw-preview" => Some(Command::OpsRawPreview),
        "ops-raw-check" => Some(Command::OpsRawCheck),
        "ops-raw-apply" => Some(Command::OpsRawApply),
        "ops-skeleton" => Some(Command::OpsSkeleton),
        "ops-schema" => Some(Command::OpsSchema),
        "ops-split" => Some(Command::OpsSplit),
        "ops-verify" => Some(Command::OpsVerify),
        "ops-summary" => Some(Command::OpsSummary),
        "range-for-symbol" => Some(Command::RangeForSymbol),
        "range-for-lines" => Some(Command::RangeForLines),
        "anchor-range" => Some(Command::AnchorRange),
        "commit-files" => Some(Command::CommitFiles),
        "explain" | "--help" | "-h" => Some(Command::Explain),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};

    #[test]
    fn parses_find_query_and_limit() {
        let cli = Cli::parse([
            "find".to_string(),
            "AssetTreePanel".to_string(),
            "--limit".to_string(),
            "12".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Find);
        assert_eq!(cli.options.query.as_deref(), Some("AssetTreePanel"));
        assert_eq!(cli.options.limit, 12);
    }

    #[test]
    fn parses_changed_group() {
        let cli = Cli::parse([
            "changed".to_string(),
            "--group".to_string(),
            "package".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Changed);
        assert_eq!(cli.options.group.as_deref(), Some("package"));
    }

    #[test]
    fn parses_files_query_and_group() {
        let cli = Cli::parse([
            "files".to_string(),
            "--query".to_string(),
            "layer:frontend,kind:test".to_string(),
            "--group".to_string(),
            "tag".to_string(),
            "--changed".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Files);
        assert_eq!(
            cli.options.query.as_deref(),
            Some("layer:frontend,kind:test")
        );
        assert_eq!(cli.options.group.as_deref(), Some("tag"));
        assert!(cli.options.changed_only);
    }

    #[test]
    fn parses_stale_patterns() {
        let cli = Cli::parse([
            "stale".to_string(),
            "--patterns".to_string(),
            "one,two".to_string(),
            "--changed".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Stale);
        assert_eq!(cli.options.patterns, vec!["one", "two"]);
        assert!(cli.options.changed_only);
    }

    #[test]
    fn parses_verify_plan_changed() {
        let cli = Cli::parse(["verify-plan".to_string(), "--changed".to_string()])
            .expect("cli should parse");

        assert_eq!(cli.command, Command::VerifyPlan);
        assert!(cli.options.changed_only);
    }

    #[test]
    fn parses_impact_group() {
        let cli = Cli::parse([
            "impact".to_string(),
            "EditorSelectionRef".to_string(),
            "--group".to_string(),
            "feature".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Impact);
        assert_eq!(cli.options.query.as_deref(), Some("EditorSelectionRef"));
        assert_eq!(cli.options.group.as_deref(), Some("feature"));
    }

    #[test]
    fn parses_fallout_from() {
        let cli = Cli::parse([
            "fallout".to_string(),
            "--from".to_string(),
            "npm-build.log".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Fallout);
        assert_eq!(
            cli.options
                .from
                .as_ref()
                .map(|path| path.display().to_string()),
            Some("npm-build.log".to_string())
        );
    }

    #[test]
    fn parses_command_map_query() {
        let cli = Cli::parse(["command-map".to_string(), "append-plan".to_string()])
            .expect("cli should parse");

        assert_eq!(cli.command, Command::CommandMap);
        assert_eq!(cli.options.query.as_deref(), Some("append-plan"));
    }

    #[test]
    fn parses_anchors_query_and_write() {
        let cli = Cli::parse([
            "anchors".to_string(),
            "domain:codemap".to_string(),
            "--write".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Anchors);
        assert_eq!(cli.options.query.as_deref(), Some("domain:codemap"));
        assert!(cli.options.write);
    }

    #[test]
    fn parses_status() {
        let cli = Cli::parse(["status".to_string()]).expect("cli should parse");

        assert_eq!(cli.command, Command::Status);
    }

    #[test]
    fn parses_refresh() {
        let cli = Cli::parse(["refresh".to_string()]).expect("cli should parse");

        assert_eq!(cli.command, Command::Refresh);
    }

    #[test]
    fn parses_changes_flags() {
        let cli = Cli::parse([
            "changes".to_string(),
            "--compact".to_string(),
            "--hide-generated".to_string(),
            "--warnings".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Changes);
        assert!(cli.options.compact);
        assert!(cli.options.hide_generated);
        assert!(cli.options.warnings);
    }

    #[test]
    fn parses_commit_plan() {
        let cli = Cli::parse(["commit-plan".to_string(), "--compact".to_string()])
            .expect("cli should parse");

        assert_eq!(cli.command, Command::CommitPlan);
        assert!(cli.options.compact);
    }

    #[test]
    fn parses_no_cache() {
        let cli = Cli::parse([
            "trace".to_string(),
            "codemap".to_string(),
            "--no-cache".to_string(),
        ])
        .expect("cli should parse");

        assert!(cli.options.no_cache);
    }

    #[test]
    fn parses_quiet_flag() {
        let cli =
            Cli::parse(["brief".to_string(), "--quiet".to_string()]).expect("cli should parse");

        assert!(cli.options.quiet);
    }

    #[test]
    fn parses_slice_line_range() {
        let cli = Cli::parse([
            "slice".to_string(),
            "src/main.rs".to_string(),
            "--lines".to_string(),
            "10:20".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Slice);
        assert!(cli.options.lines);
        assert_eq!(cli.options.line_range.as_deref(), Some("10:20"));
    }

    #[test]
    fn parses_move_plan_by() {
        let cli = Cli::parse([
            "move-plan".to_string(),
            "crates/apps/amigo-editor/src-tauri/src/commands/mod.rs".to_string(),
            "--by".to_string(),
            "tauri-command".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::MovePlan);
        assert_eq!(cli.options.by.as_deref(), Some("tauri-command"));
    }

    #[test]
    fn parses_append_plan_task() {
        let cli = Cli::parse([
            "append-plan".to_string(),
            "crates/apps/amigo-editor/src/editor-components/builtinComponents.tsx".to_string(),
            "--task".to_string(),
            "component-definition".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::AppendPlan);
        assert_eq!(
            cli.options.query.as_deref(),
            Some("crates/apps/amigo-editor/src/editor-components/builtinComponents.tsx")
        );
        assert_eq!(cli.options.task.as_deref(), Some("component-definition"));
    }

    #[test]
    fn parses_copy_plan_with_donor() {
        let cli = Cli::parse([
            "copy-plan".to_string(),
            "crates/apps/amigo-editor/src/startup/NewPanel.tsx".to_string(),
            "--from".to_string(),
            "crates/apps/amigo-editor/src/startup/ModsPanel.tsx".to_string(),
            "--task".to_string(),
            "panel".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::CopyPlan);
        assert_eq!(
            cli.options.query.as_deref(),
            Some("crates/apps/amigo-editor/src/startup/NewPanel.tsx")
        );
        assert_eq!(
            cli.options.from.as_deref(),
            Some(std::path::Path::new(
                "crates/apps/amigo-editor/src/startup/ModsPanel.tsx"
            ))
        );
        assert_eq!(cli.options.task.as_deref(), Some("panel"));
    }

    #[test]
    fn parses_file_ops_flags() {
        let cli = Cli::parse([
            "file-move-plan".to_string(),
            "crates/apps/amigo-editor/src/main.tsx".to_string(),
            "--to".to_string(),
            "crates/apps/amigo-editor/src/app/main.tsx".to_string(),
            "--changed".to_string(),
            "--radius".to_string(),
            "42".to_string(),
            "--top".to_string(),
            "12".to_string(),
            "--save".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::FileMovePlan);
        assert_eq!(
            cli.options.query.as_deref(),
            Some("crates/apps/amigo-editor/src/main.tsx")
        );
        assert_eq!(cli.options.radius, 42);
        assert_eq!(cli.options.top, 12);
        assert!(cli.options.changed_only);
        assert_eq!(
            cli.options
                .to
                .as_ref()
                .map(|path| path.display().to_string()),
            Some("crates/apps/amigo-editor/src/app/main.tsx".to_string())
        );
        assert!(cli.options.save);
    }

    #[test]
    fn parses_workset_from_impact_and_split_hints() {
        let cli = Cli::parse([
            "workset".to_string(),
            "selection-migration".to_string(),
            "--from-impact".to_string(),
            "EditorSelectionRef".to_string(),
            "--status".to_string(),
            "--with-split-hints".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Workset);
        assert_eq!(
            cli.options.from_impact.as_deref(),
            Some("EditorSelectionRef")
        );
        assert!(cli.options.status);
        assert!(cli.options.with_split_hints);
    }

    #[test]
    fn parses_patch_apply_write() {
        let cli = Cli::parse([
            "patch-apply".to_string(),
            "--from".to_string(),
            "patch.diff".to_string(),
            "--write".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::PatchApply);
        assert_eq!(
            cli.options.from.as_deref(),
            Some(std::path::Path::new("patch.diff"))
        );
        assert!(cli.options.write);
    }

    #[test]
    fn parses_where_query() {
        let cli =
            Cli::parse(["where".to_string(), "CodeMap".to_string()]).expect("cli should parse");
        assert_eq!(cli.command, Command::Where);
        assert_eq!(cli.options.query.as_deref(), Some("CodeMap"));
    }

    #[test]
    fn parses_signature_query() {
        let cli =
            Cli::parse(["signature".to_string(), "CodeMap".to_string()]).expect("cli should parse");
        assert_eq!(cli.command, Command::Signature);
        assert_eq!(cli.options.query.as_deref(), Some("CodeMap"));
    }

    #[test]
    fn parses_trace_query() {
        let cli = Cli::parse(["trace".to_string(), "entity.inspector".to_string()])
            .expect("cli should parse");
        assert_eq!(cli.command, Command::Trace);
        assert_eq!(cli.options.query.as_deref(), Some("entity.inspector"));
    }

    #[test]
    fn parses_symbols_file_metadata() {
        let cli = Cli::parse([
            "symbols".to_string(),
            "--file".to_string(),
            "crates/tools/amigo-codemap/src/scan/symbols.rs".to_string(),
            "--query".to_string(),
            "kind:fn".to_string(),
            "--metadata".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::Symbols);
        assert_eq!(cli.options.query.as_deref(), Some("kind:fn"));
        assert_eq!(
            cli.options.file.as_deref(),
            Some(std::path::Path::new(
                "crates/tools/amigo-codemap/src/scan/symbols.rs"
            ))
        );
        assert!(cli.options.metadata);
    }

    #[test]
    fn parses_ops_apply_write() {
        let cli = Cli::parse([
            "ops-apply".to_string(),
            "--from".to_string(),
            "plan.yml".to_string(),
            "--write".to_string(),
        ])
        .expect("cli should parse");
        assert_eq!(cli.command, Command::OpsApply);
        assert_eq!(
            cli.options.from.as_deref(),
            Some(std::path::Path::new("plan.yml"))
        );
        assert!(cli.options.write);
    }

    #[test]
    fn parses_ops_raw_check_from_stdin() {
        let cli = Cli::parse([
            "ops-raw-check".to_string(),
            "--from".to_string(),
            "-".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::OpsRawCheck);
        assert_eq!(cli.options.from.as_deref(), Some(std::path::Path::new("-")));
    }

    #[test]
    fn parses_ops_raw_apply_with_write() {
        let cli = Cli::parse([
            "ops-raw-apply".to_string(),
            "--from".to_string(),
            "ops.raw".to_string(),
            "--write".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::OpsRawApply);
        assert_eq!(
            cli.options.from.as_deref(),
            Some(std::path::Path::new("ops.raw"))
        );
        assert!(cli.options.write);
    }

    #[test]
    fn parses_ops_skeleton_write_out() {
        let cli = Cli::parse([
            "ops-skeleton".to_string(),
            "scan_symbols".to_string(),
            "--out".to_string(),
            "plan.yml".to_string(),
            "--write".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::OpsSkeleton);
        assert_eq!(cli.options.query.as_deref(), Some("scan_symbols"));
        assert_eq!(
            cli.options.out.file_name().and_then(|name| name.to_str()),
            Some("plan.yml")
        );
        assert!(cli.options.write);
    }

    #[test]
    fn parses_range_for_symbol_query() {
        let cli = Cli::parse(["range-for-symbol".to_string(), "scan_symbols".to_string()])
            .expect("cli should parse");

        assert_eq!(cli.command, Command::RangeForSymbol);
        assert_eq!(cli.options.query.as_deref(), Some("scan_symbols"));
    }

    #[test]
    fn parses_range_for_lines_args() {
        let cli = Cli::parse([
            "range-for-lines".to_string(),
            "src/main.ts".to_string(),
            "10".to_string(),
            "20".to_string(),
            "--yaml-op".to_string(),
            "delete_range".to_string(),
            "--context-radius".to_string(),
            "5".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::RangeForLines);
        assert_eq!(cli.options.query.as_deref(), Some("src/main.ts"));
        assert_eq!(cli.options.start_line, Some(10));
        assert_eq!(cli.options.end_line, Some(20));
        assert_eq!(cli.options.yaml_op, "delete_range");
        assert_eq!(cli.options.context_radius, 5);
    }

    #[test]
    fn parses_inline_yaml_and_strict_flags() {
        let cli = Cli::parse([
            "ops-check".to_string(),
            "--yaml".to_string(),
            "ops: []".to_string(),
            "--strict".to_string(),
            "--raw".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::OpsCheck);
        assert_eq!(cli.options.yaml.as_deref(), Some("ops: []"));
        assert!(cli.options.strict);
        assert!(cli.options.raw);
    }

    #[test]
    fn parses_ops_schema_json_example() {
        let cli = Cli::parse([
            "ops-schema".to_string(),
            "--json".to_string(),
            "--example".to_string(),
            "replace_symbol".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::OpsSchema);
        assert_eq!(cli.options.query.as_deref(), Some("replace_symbol"));
        assert!(cli.options.json);
    }

    #[test]
    fn parses_anchor_range_to() {
        let cli = Cli::parse([
            "anchor-range".to_string(),
            "tree-start".to_string(),
            "--to".to_string(),
            "tree-end".to_string(),
        ])
        .expect("cli should parse");

        assert_eq!(cli.command, Command::AnchorRange);
        assert_eq!(cli.options.query.as_deref(), Some("tree-start"));
        assert_eq!(
            cli.options.to.as_deref(),
            Some(std::path::Path::new("tree-end"))
        );
    }
}
