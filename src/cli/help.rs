use std::collections::BTreeMap;

use super::options::{COMMAND_SPECS, CommandFamily, command_family};

const FLAGS: &[(&str, &str)] = &[
    ("--root <path>", "workspace root"),
    ("--out <path>", "output directory"),
    ("--level <0-3>", "scan depth"),
    ("--pretty", "pretty output"),
    ("--ai", "AI-oriented output"),
    ("--query <text>", "explicit query"),
    ("--group <name>", "grouping key"),
    ("--start-line <n>", "start line for range operations"),
    ("--end-line <n>", "end line for range operations"),
    ("--lines", "line-aware output"),
    ("--limit <n>", "result limit"),
    ("--min-score <n>", "minimum score"),
    ("--report", "emit report"),
    ("--report-file <path>", "write report file"),
    ("--changed-only", "changed files only"),
    ("--file <path>", "target file"),
    ("--path <path>", "target file alias"),
    ("--name <name>", "exact symbol name filter"),
    ("--kind <kind>", "exact symbol kind filter"),
    ("--owner <text>", "owner filter"),
    ("--visibility <name>", "visibility filter"),
    ("--from <path>", "source path"),
    ("--to <path>", "destination path"),
    ("--symbol <name>", "target symbol"),
    ("--by <name>", "edit op kind for preview-edit/compile-edit"),
    ("--with-file <path>", "replacement content file"),
    ("--with-text <text>", "inline replacement content"),
    ("--task <name>", "task hint"),
    ("--timings", "print timings"),
    ("--progress", "print progress"),
    ("--diagnostics", "print diagnostics"),
    ("--slow-file-threshold-ms <n>", "slow file threshold"),
    ("--max-file-size <bytes>", "max file size"),
    ("--max-files <n>", "max files"),
    ("--status", "show status"),
    ("--write", "write changes"),
    ("--strict", "strict validation"),
    ("--backup", "create backups"),
    ("--stop-on-error", "stop on first error"),
    ("--run", "run generated action"),
    ("--why", "explain ranking"),
    ("--metadata", "print metadata"),
    ("--json", "JSON output"),
    ("--raw", "raw ops mode"),
    ("--no-verbose", "suppress verbose output"),
    ("--quiet", "quiet mode"),
    ("--no-cache", "bypass cache"),
    ("--compact", "compact output"),
    ("--hide-generated", "hide generated files"),
    ("--include-tests", "include tests"),
    ("--include-generated", "include generated files"),
    ("--warnings", "print warnings"),
    ("--expect-present <query>", "expect symbol/text to exist"),
    ("--expect-absent <query>", "expect symbol/text to be absent"),
    ("--daemon <mode>", "daemon mode: auto|require|disabled"),
    ("--no-daemon", "disable daemon usage"),
    ("--help, -h", "show help"),
];

pub fn print_help() {
    println!("amigo-codemap\n");
    println!("Usage:");
    println!("  amigo-codemap <command> [options] [query]\n");
    println!("Commands:");

    let mut groups = BTreeMap::<CommandFamily, Vec<&'static str>>::new();
    for spec in COMMAND_SPECS {
        groups
            .entry(command_family(spec.command))
            .or_default()
            .push(spec.name);
    }

    for (family, mut commands) in groups {
        commands.sort_unstable();
        println!("\n{}:", title_case(family.label()));
        for command in commands {
            println!("  {command:<24}");
        }
    }

    println!("\nFlags:");
    for (flag, description) in FLAGS {
        println!("  {flag:<30} {description}");
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_ascii_uppercase().to_string() + chars.as_str()
}
