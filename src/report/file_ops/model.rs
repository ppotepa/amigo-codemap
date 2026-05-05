#![allow(dead_code)]

use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileRef {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
    pub changed: bool,
}

#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub source_file: PathBuf,
    pub line: usize,
    pub raw: String,
    pub specifier: String,
    pub resolved: Option<PathBuf>,
    pub exists: bool,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    TypeScript,
    RustUse,
    RustMod,
    Css,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub source_file: PathBuf,
    pub line: usize,
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Named,
    Default,
    ReExport,
    RustPubUse,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Risk {
    pub level: RiskLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct NextAction {
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct FileOpReport {
    pub task: String,
    pub scope: Vec<String>,
    pub findings: Vec<String>,
    pub risks: Vec<Risk>,
    pub verify: Vec<String>,
    pub next: Vec<NextAction>,
}

pub fn render_report(report: &FileOpReport) -> String {
    let mut output = String::new();
    writeln!(output, "task: {}", report.task).unwrap();

    writeln!(output, "scope:").unwrap();
    if report.scope.is_empty() {
        writeln!(output, "  none").unwrap();
    } else {
        for item in &report.scope {
            writeln!(output, "  {item}").unwrap();
        }
    }

    writeln!(output, "findings:").unwrap();
    if report.findings.is_empty() {
        writeln!(output, "  none").unwrap();
    } else {
        for item in &report.findings {
            writeln!(output, "  {item}").unwrap();
        }
    }

    writeln!(output, "risk:").unwrap();
    if report.risks.is_empty() {
        writeln!(output, "  none").unwrap();
    } else {
        for risk in &report.risks {
            writeln!(output, "  {:?}: {}", risk.level, risk.message).unwrap();
        }
    }

    writeln!(output, "verify:").unwrap();
    if report.verify.is_empty() {
        writeln!(output, "  none").unwrap();
    } else {
        for item in &report.verify {
            writeln!(output, "  {item}").unwrap();
        }
    }

    writeln!(output, "next:").unwrap();
    if report.next.is_empty() {
        writeln!(output, "  1. run affected checks").unwrap();
    } else {
        for (index, action) in report.next.iter().enumerate() {
            writeln!(output, "  {}. {}", index + 1, action.label).unwrap();
        }
    }

    output
}

pub fn print_report(report: &FileOpReport) {
    print!("{}", render_report(report));
}
