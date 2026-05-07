#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSignature {
    pub signature: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub generics: Vec<String>,
    pub line_end: usize,
    pub confidence: u8,
}

pub fn extract_signature(lines: &[&str], start_index: usize, language: &str) -> ExtractedSignature {
    let raw = collect_signature_text(lines, start_index);
    let signature = normalize_signature(&raw);
    let line_end = infer_line_end(lines, start_index);
    let params = extract_params(&signature);
    let generics = extract_generics(&signature);
    let return_type = match language {
        "rs" => extract_rust_return_type(&signature),
        "ts" | "tsx" => extract_ts_return_type(&signature),
        _ => None,
    };

    ExtractedSignature {
        signature,
        params,
        return_type,
        generics,
        line_end,
        confidence: 75,
    }
}

fn collect_signature_text(lines: &[&str], start_index: usize) -> String {
    let mut result = String::new();
    let mut paren_depth = 0isize;
    let mut angle_depth = 0isize;
    let mut started = false;

    for line in lines.iter().skip(start_index).take(40) {
        let trimmed = line.trim();
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(trimmed);

        for ch in trimmed.chars() {
            match ch {
                '(' => {
                    started = true;
                    paren_depth += 1;
                }
                ')' => paren_depth -= 1,
                '<' if looks_like_generic_context(&result) => angle_depth += 1,
                '>' if angle_depth > 0 => angle_depth -= 1,
                _ => {}
            }
        }

        if started && paren_depth <= 0 && angle_depth <= 0 && signature_has_terminator(trimmed) {
            break;
        }
        if !started && signature_has_terminator(trimmed) {
            break;
        }
    }

    trim_signature_tail(&result)
}

fn infer_line_end(lines: &[&str], start_index: usize) -> usize {
    let mut brace_depth = 0isize;
    let mut seen_open_brace = false;

    for (index, line) in lines.iter().enumerate().skip(start_index) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    seen_open_brace = true;
                    brace_depth += 1;
                }
                '}' => brace_depth -= 1,
                _ => {}
            }
        }
        if seen_open_brace && brace_depth <= 0 {
            return index + 1;
        }
        if !seen_open_brace && line.trim_end().ends_with(';') {
            return index + 1;
        }
    }

    start_index + 1
}

fn signature_has_terminator(line: &str) -> bool {
    line.contains('{') || line.contains("=>") || line.ends_with(';')
}

fn trim_signature_tail(value: &str) -> String {
    let mut end = value.len();
    for marker in [" {", "{", " =>", "=>", ";"] {
        if let Some(index) = value.find(marker) {
            end = end.min(index);
        }
    }
    value[..end].trim().to_string()
}

fn normalize_signature(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" :", ":")
}

fn extract_params(signature: &str) -> Vec<String> {
    let Some(start) = signature.find('(') else {
        return Vec::new();
    };
    let Some(end) = matching_paren_end(signature, start) else {
        return Vec::new();
    };
    split_top_level(&signature[start + 1..end], ',')
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn extract_generics(signature: &str) -> Vec<String> {
    let search_end = signature.find('(').unwrap_or(signature.len());
    let Some(start) = signature[..search_end].find('<') else {
        return Vec::new();
    };
    let Some(end) = matching_angle_end(signature, start) else {
        return Vec::new();
    };
    split_top_level(&signature[start + 1..end], ',')
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn extract_rust_return_type(signature: &str) -> Option<String> {
    signature
        .split_once("->")
        .map(|(_, right)| right.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_ts_return_type(signature: &str) -> Option<String> {
    let paren_start = signature.find('(')?;
    let paren_end = matching_paren_end(signature, paren_start)?;
    signature[paren_end + 1..]
        .trim()
        .strip_prefix(':')
        .map(str::trim)
        .map(ToOwned::to_owned)
        .filter(|value| !value.is_empty())
}

fn matching_paren_end(value: &str, start: usize) -> Option<usize> {
    matching_end(value, start, '(', ')')
}

fn matching_angle_end(value: &str, start: usize) -> Option<usize> {
    matching_end(value, start, '<', '>')
}

fn matching_end(value: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0isize;
    for (index, ch) in value.char_indices().skip_while(|(index, _)| *index < start) {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn split_top_level(value: &str, separator: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0isize;
    let mut angle_depth = 0isize;

    for (index, ch) in value.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '<' => angle_depth += 1,
            '>' => angle_depth -= 1,
            _ => {}
        }
        if ch == separator && paren_depth == 0 && angle_depth == 0 {
            result.push(&value[start..index]);
            start = index + ch.len_utf8();
        }
    }
    result.push(&value[start..]);
    result
}

fn looks_like_generic_context(value: &str) -> bool {
    value.contains("fn ")
        || value.contains("function ")
        || value.contains("type ")
        || value.contains("interface ")
        || value.contains("struct ")
        || value.contains("enum ")
        || value.contains("impl")
}

#[cfg(test)]
mod tests {
    use super::extract_signature;

    #[test]
    fn extracts_multiline_ts_signature() {
        let lines = vec![
            "export function createTask<T>(",
            "  id: string,",
            "  payload: TaskPayload<T>,",
            "): EditorTask<T> {",
            "  return {} as EditorTask<T>;",
            "}",
        ];

        let signature = extract_signature(&lines, 0, "ts");

        assert_eq!(
            signature.signature,
            "export function createTask<T>(id: string, payload: TaskPayload<T>,): EditorTask<T>"
        );
        assert_eq!(signature.return_type.as_deref(), Some("EditorTask<T>"));
        assert_eq!(signature.generics, vec!["T"]);
    }

    #[test]
    fn extracts_rust_return_type() {
        let lines = vec![
            "pub fn scan_project(",
            "    options: &Options,",
            ") -> Result<CodeMap> {",
            "    todo!()",
            "}",
        ];

        let signature = extract_signature(&lines, 0, "rs");

        assert_eq!(signature.return_type.as_deref(), Some("Result<CodeMap>"));
        assert!(signature.params.join(",").contains("options: &Options"));
    }
}
