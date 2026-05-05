# amigo-codemap PR split plan

Recommended review split for the refactor-report work:

1. `refactor(codemap): split report module`
   - Move `src/report.rs` to `src/report/mod.rs`.
   - Add shared helpers in `src/report/common.rs`.

2. `feat(codemap): add verify-plan and stale reports`
   - Add `verify-plan`.
   - Add `stale`.
   - Add shared CLI flags used by both reports.

3. `feat(codemap): add impact and fallout reports`
   - Add `impact`.
   - Add `fallout`.
   - Add fallout fixtures and output snapshot.

4. `feat(codemap): add move and duplicate reports`
   - Add `move-plan`.
   - Add `dup`.
   - Add Tauri command grouping fixtures for move planning.

5. `feat(codemap): add editor-specific codemap checks`
   - Add `tauri-commands`.
   - Add `service-shape`.
   - Add `registry-check`.
   - Add registry fixtures and snapshots.

6. `feat(codemap): add operations and commit summaries`
   - Add `operations-summary`.
   - Add `commit-summary`.

7. `docs(codemap): document refactor reports`
   - Update CLI help.
   - Update README examples.
   - Keep smoke commands documented.
