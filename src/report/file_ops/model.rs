#![allow(dead_code)]

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

pub fn print_report(report: &FileOpReport) {
    println!("task: {}", report.task);

    println!("scope:");
    if report.scope.is_empty() {
        println!("  none");
    } else {
        for item in &report.scope {
            println!("  {item}");
        }
    }

    println!("findings:");
    if report.findings.is_empty() {
        println!("  none");
    } else {
        for item in &report.findings {
            println!("  {item}");
        }
    }

    println!("risk:");
    if report.risks.is_empty() {
        println!("  none");
    } else {
        for risk in &report.risks {
            println!("  {:?}: {}", risk.level, risk.message);
        }
    }

    println!("verify:");
    if report.verify.is_empty() {
        println!("  none");
    } else {
        for item in &report.verify {
            println!("  {item}");
        }
    }

    println!("next:");
    if report.next.is_empty() {
        println!("  1. run affected checks");
    } else {
        for (index, action) in report.next.iter().enumerate() {
            println!("  {}. {}", index + 1, action.label);
        }
    }
}
