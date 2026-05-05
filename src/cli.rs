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
    pub group: Option<String>,
    pub lines: bool,
    pub limit: usize,
    pub verify_args: Vec<String>,
    pub changed_only: bool,
    pub patterns: Vec<String>,
    pub from: Option<PathBuf>,
    pub by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Scan,
    Watch,
    Changed,
    Symbols,
    Compact,
    Explain,
    Brief,
    Find,
    Scope,
    Refs,
    Docs,
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
    OperationsSummary,
    CommitSummary,
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
        let mut group = None;
        let mut lines = false;
        let mut limit = 80;
        let mut verify_args = Vec::new();
        let mut changed_only = false;
        let mut patterns = Vec::new();
        let mut from = None;
        let mut by = None;

        let args = args.into_iter().collect::<Vec<_>>();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            match arg.as_str() {
                "scan" => command = Some(Command::Scan),
                "watch" => command = Some(Command::Watch),
                "changed" => command = Some(Command::Changed),
                "symbols" => command = Some(Command::Symbols),
                "compact" => command = Some(Command::Compact),
                "brief" => command = Some(Command::Brief),
                "find" => command = Some(Command::Find),
                "scope" => command = Some(Command::Scope),
                "refs" => command = Some(Command::Refs),
                "docs" | "readme-coverage" => command = Some(Command::Docs),
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
                "operations-summary" => command = Some(Command::OperationsSummary),
                "commit-summary" => command = Some(Command::CommitSummary),
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
                "--lines" => lines = true,
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
                "--from" => {
                    index += 1;
                    from = Some(PathBuf::from(required_value(&args, index, "--from")?));
                }
                "--by" => {
                    index += 1;
                    by = Some(required_value(&args, index, "--by")?);
                }
                "--limit" => {
                    index += 1;
                    limit = required_value(&args, index, "--limit")?.parse::<usize>()?;
                }
                unknown if unknown.starts_with('-') => bail!("unknown flag `{unknown}`"),
                value => match command {
                    Some(
                        Command::Find
                        | Command::Scope
                        | Command::Refs
                        | Command::Docs
                        | Command::Impact
                        | Command::MovePlan
                        | Command::Dup
                        | Command::ServiceShape
                        | Command::RegistryCheck,
                    )
                        if query.is_none() =>
                    {
                        query = Some(value.to_owned());
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
                group,
                lines,
                limit,
                verify_args,
                changed_only,
                patterns,
                from,
                by,
            },
        })
    }
}

pub fn print_help() {
    println!(
        "amigo-codemap\n\ncommands:\n  scan\n  watch\n  brief\n  compact\n  changed --group path|package|language|status\n  find <text>\n  scope <query>\n  refs <query>\n  docs\n  verify <profile>\n  verify-plan [--changed]\n  stale --patterns a,b,c [--changed]\n  impact <symbol> [--group feature|path|package]\n  fallout [--from file]\n  move-plan <file> [--by tauri-command|symbol]\n  dup [symbol] [--changed]\n  tauri-commands\n  service-shape <TypeName>\n  registry-check [properties|components|file-rules|project-actions]\n  operations-summary\n  commit-summary [--changed]\n\nflags:\n  --root <path>    project root, defaults to cwd\n  --out <path>     output path, defaults to .amigo/codemap.json\n  --level <0-3>    0 files, 1 public/export symbols, 2 local symbols, 3 relations\n  --pretty         pretty JSON\n  --ai             compact/minified JSON\n  --group <kind>   group output by path|package|language|status|feature\n  --lines          include matching lines where supported\n  --changed        focus on git changed files\n  --patterns <a,b> stale patterns\n  --from <path>    fallout input file\n  --by <kind>      move/dup strategy\n  --limit <n>      output row cap, default 80"
    );
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
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
            cli.options.from.as_ref().map(|path| path.display().to_string()),
            Some("npm-build.log".to_string())
        );
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
}
