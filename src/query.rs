#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub terms: Vec<QueryTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTerm {
    pub key: Option<String>,
    pub value: String,
    pub negated: bool,
}

pub fn descriptive_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let normalized = input.replace(['_', '-'], " ");

    for token in normalized
        .split(|char: char| !char.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() >= 3)
        .filter(|token| !is_query_stopword(token))
    {
        push_unique_token(&mut tokens, token.to_ascii_lowercase());
        for part in split_camel_token(token) {
            if part.len() >= 3 && !is_query_stopword(&part) {
                push_unique_token(&mut tokens, part);
            }
        }
    }

    for phrase in descriptive_phrases(input) {
        push_unique_token(&mut tokens, phrase);
    }

    tokens
}

impl Query {
    pub fn parse(input: Option<&str>) -> Self {
        let terms = input
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(QueryTerm::parse)
            .collect();

        Self { terms }
    }

    pub fn matches_tags(&self, tags: &[String]) -> bool {
        self.terms.iter().all(|term| {
            let matched = term.matches_tags(tags);
            if term.negated { !matched } else { matched }
        })
    }

    pub fn matches_symbol(
        &self,
        name: &str,
        kind: &str,
        visibility: &str,
        owner: Option<&str>,
        tags: &[String],
    ) -> bool {
        self.terms.iter().all(|term| {
            let matched = match term.key.as_deref() {
                Some("name") => contains_ci(name, &term.value),
                Some("kind") => eq_or_contains_ci(kind, &term.value),
                Some("visibility") => eq_or_contains_ci(visibility, &term.value),
                Some("owner") => owner.is_some_and(|value| contains_ci(value, &term.value)),
                Some(_) | None => {
                    contains_ci(name, &term.value)
                        || contains_ci(kind, &term.value)
                        || contains_ci(visibility, &term.value)
                        || owner.is_some_and(|value| contains_ci(value, &term.value))
                        || term.matches_tags(tags)
                }
            };
            if term.negated { !matched } else { matched }
        })
    }
}

impl QueryTerm {
    fn parse(raw: &str) -> Self {
        let (negated, raw) = raw
            .strip_prefix('!')
            .map(|value| (true, value))
            .unwrap_or((false, raw));

        let (key, value) = raw
            .split_once(':')
            .map(|(key, value)| {
                (
                    Some(key.trim().to_ascii_lowercase()),
                    value.trim().to_string(),
                )
            })
            .unwrap_or((None, raw.trim().to_string()));

        Self {
            key,
            value: value.to_ascii_lowercase(),
            negated,
        }
    }

    fn matches_tags(&self, tags: &[String]) -> bool {
        let needle = match self.key.as_deref() {
            Some(key) => format!("{key}:{}", self.value),
            None => self.value.clone(),
        };

        tags.iter().any(|tag| {
            let tag = tag.to_ascii_lowercase();
            if self.key.is_some() {
                tag == needle
            } else {
                tag.contains(&needle)
            }
        })
    }
}

fn contains_ci(value: &str, needle: &str) -> bool {
    value.to_ascii_lowercase().contains(needle)
}

fn eq_or_contains_ci(value: &str, needle: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value == needle || value.contains(needle)
}

fn descriptive_phrases(input: &str) -> Vec<String> {
    let lower = input.to_ascii_lowercase().replace(['_', '-'], " ");
    let phrase_rules = [
        (["ui", "document"].as_slice(), "ui-document"),
        (["scene", "editor"].as_slice(), "scene-editor"),
        (["editor", "mode"].as_slice(), "editor-mode"),
        (["main", "window"].as_slice(), "main-window"),
        (["open", "set"].as_slice(), "open-set"),
        (["change", "plan"].as_slice(), "change-plan"),
        (["node", "kind"].as_slice(), "node-kind"),
        (["scoped", "view"].as_slice(), "scoped-view"),
        (["scoped", "viewer"].as_slice(), "scoped-view"),
        (["real", "tree"].as_slice(), "real-tree"),
        (["yaml", "tree"].as_slice(), "yaml-driven"),
    ];

    phrase_rules
        .iter()
        .filter_map(|(needles, phrase)| {
            needles
                .iter()
                .all(|needle| lower.split_whitespace().any(|part| part == *needle))
                .then(|| (*phrase).to_string())
        })
        .collect()
}

fn split_camel_token(token: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut previous_was_lowercase = false;

    for char in token.chars() {
        if char.is_ascii_uppercase() && previous_was_lowercase {
            push_unique_token(&mut parts, current);
            current = String::new();
        }
        if char.is_ascii_alphanumeric() {
            previous_was_lowercase = char.is_ascii_lowercase() || char.is_ascii_digit();
            current.extend(char.to_lowercase());
        }
    }

    push_unique_token(&mut parts, current);
    parts
}

fn push_unique_token(tokens: &mut Vec<String>, token: String) {
    if !token.is_empty() && !tokens.iter().any(|item| item == &token) {
        tokens.push(token);
    }
}

fn is_query_stopword(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "and"
            | "the"
            | "for"
            | "with"
            | "from"
            | "mode"
            | "flow"
            | "query"
            | "task"
            | "real"
            | "view"
            | "viewer"
            | "node"
            | "change"
            | "changes"
    )
}

#[cfg(test)]
mod tests {
    use super::Query;

    #[test]
    fn matches_positive_tags() {
        let query = Query::parse(Some("layer:app,kind:source"));
        let tags = vec!["layer:app".to_string(), "kind:source".to_string()];
        assert!(query.matches_tags(&tags));
    }

    #[test]
    fn rejects_negated_tags() {
        let query = Query::parse(Some("layer:app,!kind:test"));
        let tags = vec!["layer:app".to_string(), "kind:test".to_string()];
        assert!(!query.matches_tags(&tags));
    }

    #[test]
    fn matches_symbol_name_and_visibility() {
        let query = Query::parse(Some("name:Editor,visibility:export"));
        assert!(query.matches_symbol(
            "EditorStore",
            "type",
            "export",
            None,
            &["domain:workspace".to_string()]
        ));
    }

    #[test]
    fn descriptive_tokens_include_domain_phrases() {
        let tokens =
            super::descriptive_tokens("ui document real yaml tree icons scoped node viewer");

        assert!(tokens.iter().any(|token| token == "ui-document"));
        assert!(tokens.iter().any(|token| token == "yaml-driven"));
        assert!(tokens.iter().any(|token| token == "scoped-view"));
        assert!(tokens.iter().any(|token| token == "icons"));
    }
}
