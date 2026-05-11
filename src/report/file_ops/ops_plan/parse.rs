use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use serde_yaml::Value;

use super::model::{OpsEntry, OpsInputFormat, OpsPlan};
use super::path_safety::validate_repo_relative_path;

pub(super) fn read_plan_with_format(
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: OpsInputFormat,
) -> Result<OpsPlan> {
    let mut plan_dir = None;
    let text = if let Some(yaml) = yaml {
        yaml.to_string()
    } else {
        let path = from.ok_or_else(|| {
            anyhow!("ops plan requires --from <plan.yml>, --from -, or --yaml <text>")
        })?;
        if path == Path::new("-") {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text)?;
            text
        } else {
            plan_dir = path.parent().map(Path::to_path_buf);
            fs::read_to_string(path)?
        }
    };

    let mut plan = match input_format {
        OpsInputFormat::Yaml => {
            validate_plan_shape(&text)?;
            serde_yaml::from_str(&text)?
        }
        OpsInputFormat::Raw => parse_raw_ops_plan(&text)?,
    };
    if plan.version != 1 {
        bail!("unsupported ops plan version {}; expected 1", plan.version);
    }
    if let Some(content_root) = &plan.content_root {
        validate_repo_relative_path(content_root)?;
    }
    plan.plan_dir = plan_dir;
    Ok(plan)
}

#[cfg(test)]
pub(super) fn read_plan(from: Option<&Path>, yaml: Option<&str>) -> Result<OpsPlan> {
    read_plan_with_format(from, yaml, OpsInputFormat::Yaml)
}

fn parse_raw_ops_plan(text: &str) -> Result<OpsPlan> {
    let mut blocks = Vec::new();
    let mut current = RawOpsBlock::default();
    let mut multiline_field = None::<String>;
    let mut saw_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed == "END" {
            if let Some(field) = multiline_field.take() {
                current.finish_multiline(&field);
            }
            if saw_block && current.action.is_some() {
                blocks.push(std::mem::take(&mut current));
                saw_block = false;
            }
            continue;
        }

        if let Some(field) = multiline_field.as_deref() {
            if is_raw_key_line(line) {
                let field = multiline_field.take().unwrap();
                current.finish_multiline(&field);
            } else {
                current.push_multiline(field, line);
                continue;
            }
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            bail!("raw ops line must use KEY: value format: `{trimmed}`");
        };
        let key = key.trim().to_ascii_uppercase();
        let value = value.trim_start().to_owned();
        if key.eq_ignore_ascii_case("ACTION") && saw_block && current.action.is_some() {
            blocks.push(std::mem::take(&mut current));
        }
        saw_block = true;

        if value.is_empty() && matches!(key.as_str(), "CONTENT" | "FIND" | "REPLACE") {
            multiline_field = Some(key);
            continue;
        }

        match key.as_str() {
            "ACTION" => current.action = Some(value),
            "FILE" => current.file = Some(value),
            "SYMBOL" => current.symbol = Some(value),
            "WITHIN_SYMBOL" => current.within_symbol = Some(value),
            "FIND" => current.find = Some(value),
            "REPLACE" => current.replace = Some(value),
            "TEXT" => current.text = Some(value),
            "START_LINE" => current.start_line = Some(value.parse()?),
            "END_LINE" => current.end_line = Some(value.parse()?),
            "EXPECTED_HASH" => current.expected_hash = Some(value),
            "EXPECTED_MATCHES" => current.expected_matches = Some(value.parse()?),
            "SCOPE" => current.scope = Some(value),
            "CHANGED_ONLY" => current.changed_only = value.parse::<bool>()?,
            other => bail!("unknown raw ops field `{other}`"),
        }
    }

    if let Some(field) = multiline_field.take() {
        bail!("raw ops {field} block is missing END");
    }
    if saw_block && current.action.is_some() {
        blocks.push(current);
    }
    if blocks.is_empty() {
        bail!("raw ops input did not contain any ACTION blocks");
    }

    let ops = blocks
        .into_iter()
        .enumerate()
        .map(|(index, block)| block.into_ops_entry(index + 1))
        .collect::<Result<Vec<_>>>()?;

    Ok(OpsPlan {
        version: 1,
        task: Some("raw-ops-plan".to_owned()),
        description: Some("Parsed from raw ACTION blocks".to_owned()),
        content_root: None,
        ops,
        verify: Vec::new(),
        plan_dir: None,
    })
}

fn is_raw_key_line(line: &str) -> bool {
    let Some((key, _)) = line.trim_start().split_once(':') else {
        return false;
    };
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == '_' || ch == '-')
}

#[derive(Debug, Default)]
struct RawOpsBlock {
    action: Option<String>,
    file: Option<String>,
    symbol: Option<String>,
    within_symbol: Option<String>,
    find: Option<String>,
    replace: Option<String>,
    text: Option<String>,
    content: String,
    start_line: Option<usize>,
    end_line: Option<usize>,
    expected_hash: Option<String>,
    expected_matches: Option<usize>,
    scope: Option<String>,
    changed_only: bool,
}

impl RawOpsBlock {
    fn push_multiline(&mut self, field: &str, line: &str) {
        match field {
            "CONTENT" => {
                self.content.push_str(line);
                self.content.push('\n');
            }
            "FIND" => {
                let value = self.find.get_or_insert_with(String::new);
                value.push_str(line);
                value.push('\n');
            }
            "REPLACE" => {
                let value = self.replace.get_or_insert_with(String::new);
                value.push_str(line);
                value.push('\n');
            }
            _ => {}
        }
    }

    fn finish_multiline(&mut self, field: &str) {
        match field {
            "CONTENT" => {}
            "FIND" => {
                if let Some(value) = &mut self.find {
                    *value = value.trim_end().to_string();
                }
            }
            "REPLACE" => {
                if let Some(value) = &mut self.replace {
                    *value = value.trim_end().to_string();
                }
            }
            _ => {}
        }
    }

    fn into_ops_entry(self, index: usize) -> Result<OpsEntry> {
        let action = self
            .action
            .as_deref()
            .ok_or_else(|| anyhow!("raw ops block {index} is missing ACTION"))?
            .trim()
            .replace('_', " ")
            .to_ascii_uppercase();
        let path = self
            .file
            .as_deref()
            .ok_or_else(|| anyhow!("raw ops block {index} is missing FILE"))?;
        let path = PathBuf::from(path);
        let content = if self.content.is_empty() {
            None
        } else {
            Some(self.content)
        };

        match action.as_str() {
            "CREATE FILE" => Ok(OpsEntry::CreateFile {
                id: None,
                path,
                content,
                content_from: None,
                overwrite: false,
            }),
            "REPLACE SYMBOL" => Ok(OpsEntry::ReplaceSymbol {
                id: None,
                path,
                symbol: required_raw_field(index, "SYMBOL", self.symbol)?,
                content,
                content_from: None,
                expected_hash: self.expected_hash,
                context_before: None,
                context_after: None,
            }),
            "INSERT BEFORE SYMBOL" => Ok(OpsEntry::InsertBeforeSymbol {
                id: None,
                path,
                symbol: required_raw_field(index, "SYMBOL", self.symbol)?,
                content,
                content_from: None,
                expected_hash: self.expected_hash,
            }),
            "INSERT AFTER SYMBOL" => Ok(OpsEntry::InsertAfterSymbol {
                id: None,
                path,
                symbol: required_raw_field(index, "SYMBOL", self.symbol)?,
                content,
                content_from: None,
                expected_hash: self.expected_hash,
            }),
            "REPLACE TEXT" => Ok(OpsEntry::ReplaceText {
                id: None,
                path,
                find: required_raw_field(index, "FIND", self.find)?,
                replace: self.replace.or(content),
                content_from: None,
                within_symbol: self.within_symbol,
                expected_matches: self.expected_matches,
            }),
            "INSERT BEFORE TEXT" | "INSERT" => Ok(OpsEntry::InsertBeforeText {
                id: None,
                path,
                find: required_raw_field(index, "FIND", self.find)?,
                content,
                content_from: None,
            }),
            "INSERT AFTER TEXT" => Ok(OpsEntry::InsertAfterText {
                id: None,
                path,
                find: required_raw_field(index, "FIND", self.find)?,
                content,
                content_from: None,
            }),
            "REPLACE RANGE" => Ok(OpsEntry::ReplaceRange {
                id: None,
                path,
                start_line: self
                    .start_line
                    .ok_or_else(|| anyhow!("raw ops block {index} is missing START_LINE"))?,
                end_line: self
                    .end_line
                    .ok_or_else(|| anyhow!("raw ops block {index} is missing END_LINE"))?,
                content,
                content_from: None,
                expected_hash: self.expected_hash,
                context_before: None,
                context_after: None,
            }),
            "DELETE RANGE" => Ok(OpsEntry::DeleteRange {
                id: None,
                path,
                start_line: self
                    .start_line
                    .ok_or_else(|| anyhow!("raw ops block {index} is missing START_LINE"))?,
                end_line: self
                    .end_line
                    .ok_or_else(|| anyhow!("raw ops block {index} is missing END_LINE"))?,
                expected_hash: self.expected_hash,
                context_before: None,
                context_after: None,
            }),
            "DELETE SYMBOL IF EXISTS" => Ok(OpsEntry::DeleteSymbolIfExists {
                id: None,
                path,
                symbol: required_raw_field(index, "SYMBOL", self.symbol)?,
                expected_hash: self.expected_hash,
            }),
            "ASSERT SYMBOL ABSENT" => Ok(OpsEntry::AssertSymbolAbsent {
                id: None,
                path: self.file.map(PathBuf::from),
                symbol: required_raw_field(index, "SYMBOL", self.symbol)?,
            }),
            "ASSERT TEXT ABSENT" => Ok(OpsEntry::AssertTextAbsent {
                id: None,
                path: self.file.map(PathBuf::from),
                text: required_raw_field(index, "TEXT", self.text.or(self.find).or(self.replace))?,
                changed_only: self.changed_only,
            }),
            "MODIFY ENUM" | "MODIFY MATCH" => Ok(OpsEntry::ReplaceText {
                id: None,
                path,
                find: required_raw_field(index, "FIND", self.find)?,
                replace: self.replace.or(content),
                content_from: None,
                within_symbol: self.within_symbol,
                expected_matches: self.expected_matches,
            }),
            "REPLACE FIELD ACCESS" => Ok(OpsEntry::ReplaceFieldAccess {
                id: None,
                path: self.file.map(PathBuf::from),
                find: required_raw_field(index, "FIND", self.find)?,
                replace: required_raw_field(index, "REPLACE", self.replace.or(content))?,
                scope: self.scope,
                expected_matches: self.expected_matches,
            }),
            _ => bail!("unsupported raw ops ACTION `{action}` in block {index}"),
        }
    }
}

fn required_raw_field(index: usize, name: &str, value: Option<String>) -> Result<String> {
    value.ok_or_else(|| anyhow!("raw ops block {index} is missing {name}"))
}

fn validate_plan_shape(text: &str) -> Result<()> {
    let value: Value = serde_yaml::from_str(text)
        .map_err(|error| anyhow!("failed to parse ops plan YAML: {error}"))?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| anyhow!("ops plan must be a YAML mapping with top-level `ops`"))?;

    if !mapping.contains_key(&Value::String("ops".to_string())) {
        bail!("ops plan is missing top-level `ops`");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{OpsEntry, OpsInputFormat, read_plan, read_plan_with_format};

    #[test]
    fn read_plan_accepts_versionless_ops_plan() {
        let plan = read_plan(None, Some("ops: []\n")).expect("versionless plan should parse");

        assert_eq!(plan.version, 1);
        assert!(plan.ops.is_empty());
    }

    #[test]
    fn read_plan_accepts_explicit_version_one() {
        let plan = read_plan(None, Some("version: 1\nops: []\n")).expect("v1 plan should parse");

        assert_eq!(plan.version, 1);
    }

    #[test]
    fn read_plan_rejects_unknown_version() {
        let error = read_plan(None, Some("version: 2\nops: []\n"))
            .expect_err("unknown version should fail");

        assert!(error.to_string().contains("unsupported ops plan version 2"));
    }

    #[test]
    fn read_plan_reports_missing_ops_clearly() {
        let error =
            read_plan(None, Some("task: missing-ops\n")).expect_err("missing ops should fail");

        assert!(error.to_string().contains("missing top-level `ops`"));
    }

    #[test]
    fn read_plan_reports_non_mapping_clearly() {
        let error = read_plan(None, Some("- kind: replace_text\n"))
            .expect_err("non-mapping plan should fail");

        assert!(error.to_string().contains("must be a YAML mapping"));
    }

    #[test]
    fn raw_ops_plan_parses_replace_symbol_block() {
        let plan = read_plan_with_format(
            None,
            Some(
                "ACTION: REPLACE SYMBOL\nFILE: src/lib.rs\nSYMBOL: run\nCONTENT:\nfn run() {}\nEND\n",
            ),
            OpsInputFormat::Raw,
        )
        .expect("raw ops plan should parse");

        assert_eq!(plan.version, 1);
        assert_eq!(plan.ops.len(), 1);
        match &plan.ops[0] {
            OpsEntry::ReplaceSymbol {
                path,
                symbol,
                content,
                ..
            } => {
                assert_eq!(path.to_string_lossy(), "src/lib.rs");
                assert_eq!(symbol, "run");
                assert_eq!(content.as_deref(), Some("fn run() {}\n"));
            }
            other => panic!("expected replace_symbol, got {other:?}"),
        }
    }

    #[test]
    fn raw_ops_plan_parses_scoped_replace_text_block() {
        let plan = read_plan_with_format(
            None,
            Some(
                "ACTION: replace_text\nFILE: src/lib.rs\nWITHIN_SYMBOL: run\nEXPECTED_MATCHES: 1\nFIND:\nfont_size: 14.0,\nREPLACE:\nfont_size: theme.font.output_size,\nEND\n",
            ),
            OpsInputFormat::Raw,
        )
        .expect("raw ops plan should parse");

        match &plan.ops[0] {
            OpsEntry::ReplaceText {
                find,
                replace,
                within_symbol,
                expected_matches,
                ..
            } => {
                assert_eq!(find, "font_size: 14.0,");
                assert_eq!(
                    replace.as_deref(),
                    Some("font_size: theme.font.output_size,")
                );
                assert_eq!(within_symbol.as_deref(), Some("run"));
                assert_eq!(*expected_matches, Some(1));
            }
            other => panic!("expected replace_text, got {other:?}"),
        }
    }

    #[test]
    fn raw_ops_plan_rejects_missing_end() {
        let error = read_plan_with_format(
            None,
            Some("ACTION: CREATE FILE\nFILE: src/lib.rs\nCONTENT:\nmod api;\n"),
            OpsInputFormat::Raw,
        )
        .expect_err("unterminated content should fail");

        assert!(error.to_string().contains("missing END"));
    }
}
