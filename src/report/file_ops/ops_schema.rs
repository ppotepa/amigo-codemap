use anyhow::Result;
use serde_json::json;

pub fn print_ops_schema(example: Option<&str>, json_output: bool) -> Result<()> {
    let kinds = [
        (
            "create_file",
            &["path"][..],
            &["id", "content", "content_from"][..],
        ),
        (
            "replace_file",
            &["path"][..],
            &["id", "content", "content_from", "expected_hash"][..],
        ),
        ("delete_file", &["path"][..], &["id", "expected_hash"][..]),
        (
            "copy_file",
            &["from", "to"][..],
            &["id", "expected_hash", "overwrite"][..],
        ),
        (
            "move_file",
            &["from", "to"][..],
            &["id", "expected_hash", "overwrite"][..],
        ),
        (
            "rename_file",
            &["from", "to"][..],
            &["id", "expected_hash", "overwrite"][..],
        ),
        ("create_dir", &["path"][..], &["id"][..]),
        ("delete_dir", &["path"][..], &["id", "recursive"][..]),
        (
            "insert_before_anchor",
            &["path", "anchor"][..],
            &["id", "content", "content_from"][..],
        ),
        (
            "insert_after_anchor",
            &["path", "anchor"][..],
            &["id", "content", "content_from"][..],
        ),
        (
            "insert_before_text",
            &["path", "find"][..],
            &["id", "content", "content_from"][..],
        ),
        (
            "insert_after_text",
            &["path", "find"][..],
            &["id", "content", "content_from"][..],
        ),
        (
            "replace_text",
            &["path", "find"][..],
            &[
                "id",
                "replace",
                "content_from",
                "within_symbol",
                "expected_matches",
            ][..],
        ),
        (
            "replace_between_anchors",
            &["path", "start_anchor", "end_anchor"][..],
            &["id", "content", "content_from", "expected_hash"][..],
        ),
        (
            "replace_symbol",
            &["path", "symbol"][..],
            &[
                "id",
                "content",
                "content_from",
                "expected_hash",
                "context_before",
                "context_after",
            ][..],
        ),
        (
            "delete_symbol",
            &["path", "symbol"][..],
            &["id", "expected_hash"][..],
        ),
        (
            "insert_before_symbol",
            &["path", "symbol"][..],
            &["id", "content", "content_from", "expected_hash"][..],
        ),
        (
            "insert_after_symbol",
            &["path", "symbol"][..],
            &["id", "content", "content_from", "expected_hash"][..],
        ),
        (
            "replace_method_body",
            &["path", "symbol"][..],
            &["id", "content", "content_from", "expected_hash"][..],
        ),
        (
            "replace_range",
            &["path", "start_line", "end_line"][..],
            &[
                "id",
                "content",
                "content_from",
                "expected_hash",
                "context_before",
                "context_after",
            ][..],
        ),
        (
            "delete_range",
            &["path", "start_line", "end_line"][..],
            &["id", "expected_hash", "context_before", "context_after"][..],
        ),
        (
            "append_to_file",
            &["path"][..],
            &["id", "content", "content_from", "expected_hash"][..],
        ),
    ];

    let selected = example.and_then(|name| kinds.iter().find(|(kind, _, _)| *kind == name));
    let items = selected
        .map(|item| vec![*item])
        .unwrap_or_else(|| kinds.to_vec());

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "plan_fields": {
                    "required": ["ops"],
                    "optional": ["version", "task", "description", "content_root", "verify"],
                    "defaults": {
                        "version": 1
                    }
                },
                "ops": items.iter().map(|(kind, required, optional)| json!({
                    "kind": kind,
                    "required": required,
                    "optional": optional,
                })).collect::<Vec<_>>()
            }))?
        );
        return Ok(());
    }

    println!("plan_fields:");
    println!("  required: [ops]");
    println!("  optional: [version, task, description, content_root, verify]");
    println!("  defaults: version=1");
    println!("  note: content ops require exactly one of content/replace or content_from.");
    println!("ops:");
    for (kind, required, optional) in items {
        println!("  - kind: {kind}");
        println!("    required: [{}]", required.join(", "));
        println!("    optional: [{}]", optional.join(", "));
    }
    if example.is_none() || matches!(example, Some("replace_range" | "delete_range")) {
        println!("examples:");
        println!("  replace_range:");
        println!("    kind: replace_range");
        println!("    path: src/example.ts");
        println!("    start_line: 10");
        println!("    end_line: 12");
        println!("    expected_hash: \"abc123\"");
        println!("    context_before: |");
        println!("      function before() {{}}");
        println!("    context_after: |");
        println!("      function after() {{}}");
        println!("    content: |");
        println!("      replacement();");
        println!("  delete_range:");
        println!("    kind: delete_range");
        println!("    path: src/example.ts");
        println!("    start_line: 20");
        println!("    end_line: 24");
        println!("    expected_hash: \"abc123\"");
        println!("    context_before: |");
        println!("      const before = true;");
        println!("    context_after: |");
        println!("      const after = true;");
    }
    println!("raw_ops:");
    println!("  commands: ops-raw-preview, ops-raw-check, ops-raw-apply");
    println!("  format:");
    println!("    ACTION: replace_text");
    println!("    FILE: crates/apps/app/src/dev_console/overlay.rs");
    println!("    WITHIN_SYMBOL: console_panel_node");
    println!("    EXPECTED_MATCHES: 1");
    println!("    FIND:");
    println!("    font_size: 14.0,");
    println!("    REPLACE:");
    println!("    font_size: theme.font.output_size,");
    println!("    END");
    Ok(())
}
