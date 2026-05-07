// @codemap anchor:codemap-report-tauri-graph domain:codemap role:tauri-graph priority:P1 layer:tool tags:tauri,report
use crate::model::CodeMap;

pub fn print_tauri_graph(map: &CodeMap, limit: usize) {
    print!("{}", render_tauri_graph(map, limit));
}

pub fn render_tauri_graph(map: &CodeMap, limit: usize) -> String {
    let mut output = String::new();
    let frontend_flow = map
        .text_occurrences
        .iter()
        .filter(|occurrence| is_tauri_frontend_flow(map, occurrence))
        .take(limit)
        .collect::<Vec<_>>();
    let backend_commands = map
        .symbols
        .iter()
        .filter(|symbol| symbol.tags.iter().any(|tag| tag == "domain:tauri-commands"))
        .take(limit)
        .collect::<Vec<_>>();

    output.push_str("tauri-graph:\n");

    output.push_str("frontend invokes:\n");
    if frontend_flow.is_empty() {
        output.push_str("  none\n");
    } else {
        for occurrence in frontend_flow {
            output.push_str(&format!(
                "  {}:{} {}\n",
                file_path_for(map, &occurrence.file_id),
                occurrence.line,
                occurrence.context.trim()
            ));
        }
    }

    output.push_str("backend commands:\n");
    if backend_commands.is_empty() {
        output.push_str("  none\n");
    } else {
        for symbol in backend_commands {
            output.push_str(&format!(
                "  {} {} {}\n",
                symbol.kind, symbol.name, symbol.signature
            ));
        }
    }

    output
}

fn is_tauri_frontend_flow(map: &CodeMap, occurrence: &crate::model::TextOccurrenceEntry) -> bool {
    occurrence_path_contains(map, occurrence, "crates/apps/")
        && (occurrence.value.contains("invoke")
            || occurrence.context.contains("invoke(")
            || occurrence.context.contains("invoke<"))
}

fn file_path_for(map: &CodeMap, file_id: &str) -> String {
    map.files
        .iter()
        .find(|file| file.id == file_id)
        .map(|file| file.path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| file_id.to_owned())
}

fn occurrence_path_contains(
    map: &CodeMap,
    occurrence: &crate::model::TextOccurrenceEntry,
    needle: &str,
) -> bool {
    file_path_for(map, &occurrence.file_id).contains(needle)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::model::{
        AreaEntry, CodeMap, DependencyEntry, FileEntry, GitInfo, PackageEntry, RelationEntry,
        SymbolEntry, TextOccurrenceEntry,
    };

    use super::render_tauri_graph;

    #[test]
    fn excludes_fixture_command_names_from_frontend_flow() {
        let map = CodeMap {
            root_name: "fixture".to_owned(),
            stats: BTreeMap::new(),
            files: vec![
                file("tests/fixtures/demo.rs"),
                file("crates/apps/amigo-editor/src/api/editorApi.ts"),
            ],
            packages: Vec::<PackageEntry>::new(),
            symbols: vec![SymbolEntry {
                name: "get_project_tree".to_owned(),
                kind: "fn".to_owned(),
                file_id: "commands/project_tree.rs".to_owned(),
                line: 10,
                line_end: 12,
                line_count: 3,
                signature: "pub fn get_project_tree() -> Result<(), String>".to_owned(),
                params: vec![],
                return_type: Some("Result<(), String>".to_owned()),
                generics: vec![],
                visibility: "pub".to_owned(),
                owner: None,
                tags: vec!["domain:tauri-commands".to_owned()],
                confidence: 90,
            }],
            text_occurrences: vec![
                TextOccurrenceEntry {
                    id: "fixture-command".to_owned(),
                    value: "player".to_owned(),
                    normalized_value: "player".to_owned(),
                    kind: "command-name".to_owned(),
                    file_id: "tests/fixtures/demo.rs".to_owned(),
                    line: 5,
                    column: 1,
                    owner: None,
                    context: "entity_name: \"player\".to_owned(),".to_owned(),
                    tags: vec![],
                    confidence: 50,
                },
                TextOccurrenceEntry {
                    id: "frontend-invoke".to_owned(),
                    value: "invoke".to_owned(),
                    normalized_value: "invoke".to_owned(),
                    kind: "string-literal".to_owned(),
                    file_id: "crates/apps/amigo-editor/src/api/editorApi.ts".to_owned(),
                    line: 42,
                    column: 3,
                    owner: None,
                    context: "invoke<EditorProjectTreeDto>(\"get_project_tree\", { sessionId })"
                        .to_owned(),
                    tags: vec![],
                    confidence: 80,
                },
            ],
            tags: vec![],
            dependencies: Vec::<DependencyEntry>::new(),
            relations: Vec::<RelationEntry>::new(),
            areas: Vec::<AreaEntry>::new(),
            git: GitInfo {
                branch: "main".to_owned(),
                rev: "test".to_owned(),
                dirty: false,
                changed: vec![],
            },
        };

        let report = render_tauri_graph(&map, 20);

        assert!(report.contains("src/api/editorApi.ts:42"));
        assert!(report.contains("fn get_project_tree"));
        assert!(!report.contains("entity_name"));
        assert!(!report.contains("tests/fixtures/demo.rs"));
    }

    fn file(path: &str) -> FileEntry {
        FileEntry {
            id: path.to_owned(),
            path: path.into(),
            language: "unknown".to_owned(),
            lines: 1,
            hash: "hash".to_owned(),
            size: 1,
            tags: vec![],
        }
    }
}
