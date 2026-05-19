use std::path::PathBuf;

use anyhow::{Result, bail};

use super::command_names::parse_command_name;
use super::{Cli, DaemonMode, Options};
use crate::cli::{Command, PositionalMode, command_positional_mode};

pub(super) fn parse<I>(args: I) -> Result<Cli>
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
    let mut min_score = 0usize;
    let mut report = false;
    let mut report_file = None;
    let mut file_lines = 450usize;
    let mut verify_args = Vec::new();
    let mut changed_only = false;
    let mut patterns = Vec::new();
    let mut file = None;
    let mut name = None;
    let mut kind = None;
    let mut owner = None;
    let mut visibility = None;
    let mut from = None;
    let mut yaml = None;
    let mut yaml_op = "replace_range".to_string();
    let mut by = None;
    let mut to = None;
    let mut symbol = None;
    let mut with_file = None;
    let mut with_text = None;
    let mut task = None;
    let mut from_impact = None;
    let mut radius = 32usize;
    let mut context_radius = 3usize;
    let mut top = 20usize;
    let mut with_split_hints = false;
    let mut timings = false;
    let mut progress = false;
    let mut diagnostics = false;
    let mut slow_file_threshold_ms = 100u64;
    let mut max_file_size_bytes = 1_500_000u64;
    let mut max_files = 20_000usize;
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
    let mut include_tests = false;
    let mut include_generated = false;
    let mut warnings = false;
    let mut expect_present = Vec::new();
    let mut expect_absent = Vec::new();
    let mut daemon_mode = DaemonMode::Auto;

    let args = args.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if command.is_some() && !arg.starts_with('-') && parse_command_name(arg).is_some() {
            consume_positional(
                command,
                arg,
                &mut query,
                &mut verify_args,
                &mut start_line,
                &mut end_line,
            )?;
            index += 1;
            continue;
        }

        match arg.as_str() {
            value if parse_command_name(value).is_some() => {
                command = parse_command_name(value);
            }
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
            "--start-line" => {
                index += 1;
                start_line = Some(required_value(&args, index, "--start-line")?.parse::<usize>()?);
            }
            "--end-line" => {
                index += 1;
                end_line = Some(required_value(&args, index, "--end-line")?.parse::<usize>()?);
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
            "--path" => {
                index += 1;
                file = Some(PathBuf::from(required_value(&args, index, "--path")?));
            }
            "--name" => {
                index += 1;
                name = Some(required_value(&args, index, "--name")?);
            }
            "--kind" => {
                index += 1;
                kind = Some(required_value(&args, index, "--kind")?);
            }
            "--owner" => {
                index += 1;
                owner = Some(required_value(&args, index, "--owner")?);
            }
            "--visibility" => {
                index += 1;
                visibility = Some(required_value(&args, index, "--visibility")?);
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
            "--with-file" => {
                index += 1;
                with_file = Some(PathBuf::from(required_value(&args, index, "--with-file")?));
            }
            "--with-text" => {
                index += 1;
                with_text = Some(required_value(&args, index, "--with-text")?);
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
            "--min-score" => {
                index += 1;
                min_score = required_value(&args, index, "--min-score")?.parse::<usize>()?;
            }
            "--file-lines" => {
                index += 1;
                file_lines = required_value(&args, index, "--file-lines")?.parse::<usize>()?;
            }
            "--report" => report = true,
            "--report-file" => {
                index += 1;
                report_file = Some(PathBuf::from(required_value(
                    &args,
                    index,
                    "--report-file",
                )?));
            }
            "--with-split-hints" => with_split_hints = true,
            "--timings" => timings = true,
            "--progress" => progress = true,
            "--print" => {
                progress = true;
                diagnostics = true;
            }
            "--diagnostics" => diagnostics = true,
            "--slow-file-threshold-ms" => {
                index += 1;
                slow_file_threshold_ms =
                    required_value(&args, index, "--slow-file-threshold-ms")?.parse::<u64>()?;
            }
            "--max-file-size" => {
                index += 1;
                max_file_size_bytes =
                    required_value(&args, index, "--max-file-size")?.parse::<u64>()?;
            }
            "--max-files" => {
                index += 1;
                max_files = required_value(&args, index, "--max-files")?.parse::<usize>()?;
            }
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
            "--include-tests" => include_tests = true,
            "--include-generated" => include_generated = true,
            "--warnings" => warnings = true,
            "--daemon" => {
                index += 1;
                daemon_mode = match required_value(&args, index, "--daemon")?
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "auto" => DaemonMode::Auto,
                    "require" => DaemonMode::Require,
                    "disabled" => DaemonMode::Disabled,
                    other => bail!(
                        "unknown --daemon mode `{other}`; expected auto, require, or disabled"
                    ),
                };
            }
            "--no-daemon" => daemon_mode = DaemonMode::Disabled,
            "--expect-present" => {
                index += 1;
                expect_present.push(required_value(&args, index, "--expect-present")?);
            }
            "--expect-absent" => {
                index += 1;
                expect_absent.push(required_value(&args, index, "--expect-absent")?);
            }
            unknown if unknown.starts_with('-') => bail!("unknown flag `{unknown}`"),
            value => consume_positional(
                command,
                value,
                &mut query,
                &mut verify_args,
                &mut start_line,
                &mut end_line,
            )?,
        }
        index += 1;
    }

    let root = root.canonicalize().unwrap_or(root);
    let out = out.unwrap_or_else(|| root.join(".amigo").join("codemap.json"));

    Ok(Cli {
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
            min_score,
            report,
            report_file,
            file_lines,
            verify_args,
            changed_only,
            patterns,
            file,
            name,
            kind,
            owner,
            visibility,
            from,
            yaml,
            yaml_op,
            by,
            to,
            symbol,
            with_file,
            with_text,
            task,
            from_impact,
            radius,
            context_radius,
            top,
            with_split_hints,
            timings,
            progress,
            diagnostics,
            slow_file_threshold_ms,
            max_file_size_bytes,
            max_files,
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
            include_tests,
            include_generated,
            warnings,
            expect_present,
            expect_absent,
            daemon_mode,
        },
    })
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn consume_positional(
    command: Option<Command>,
    value: &str,
    query: &mut Option<String>,
    verify_args: &mut Vec<String>,
    start_line: &mut Option<usize>,
    end_line: &mut Option<usize>,
) -> Result<()> {
    match command.map(command_positional_mode) {
        Some(PositionalMode::Query) if query.is_none() => {
            *query = Some(value.to_owned());
            Ok(())
        }
        Some(PositionalMode::VerifyArgs) => {
            verify_args.push(value.to_owned());
            Ok(())
        }
        Some(PositionalMode::RangeLines) if start_line.is_none() => {
            *start_line = Some(value.parse::<usize>()?);
            Ok(())
        }
        Some(PositionalMode::RangeLines) if end_line.is_none() => {
            *end_line = Some(value.parse::<usize>()?);
            Ok(())
        }
        Some(PositionalMode::Query) => bail!("unexpected positional `{value}`"),
        Some(PositionalMode::RangeLines) => bail!("unexpected extra range line `{value}`"),
        Some(PositionalMode::None) | None => bail!("unknown command `{value}`"),
    }
}
