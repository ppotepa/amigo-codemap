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
}
