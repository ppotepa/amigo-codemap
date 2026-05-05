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
                "--limit" => {
                    index += 1;
                    limit = required_value(&args, index, "--limit")?.parse::<usize>()?;
                }
                unknown if unknown.starts_with('-') => bail!("unknown flag `{unknown}`"),
                value => match command {
                    Some(Command::Find | Command::Scope | Command::Refs | Command::Docs)
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
            },
        })
    }
}

pub fn print_help() {
    println!(
        "amigo-codemap\n\ncommands:\n  scan      write .amigo/codemap.json once\n  watch     update codemap after filesystem changes\n  compact   write compact AI JSON\n  brief     print tiny repo summary\n  changed   print git changed files, optionally --group path|package|language|status\n  symbols   print detected symbols\n  find      find text in indexed files\n  scope     print small context for a file, area, package, or symbol\n  refs      find definitions and text references\n  docs      print README coverage for workspace packages\n  verify    run capped checks: npm-build|npm-test|cargo-check|cargo-test\n  explain   print this help\n\nflags:\n  --root <path>    project root, defaults to cwd\n  --out <path>     output path, defaults to .amigo/codemap.json\n  --level <0-3>    0 files, 1 public/export symbols, 2 local symbols, 3 relations\n  --pretty         pretty JSON\n  --ai             compact/minified JSON\n  --group <kind>   group changed output\n  --lines          include matching lines for find/refs\n  --limit <n>      output row cap, default 80"
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
}
