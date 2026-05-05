# amigo-codemap PR split plan

`PR split` = dzielenie zmian na mniejsze pull requesty/komentarze po jednym zakresie, żeby:

- zmniejszyć ryzyko regresji,
- łatwiej robić review,
- szybciej przywracać działające kawałki, jeśli coś pójdzie nie tak.

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

7. `feat(codemap): add file-ops foundation`
   - Add `file_ops/` module and CLI wiring.
   - Add shared file-ops structures and command parser flags.

8. `feat(codemap): add file-ops context tools`
   - Add `slice`, `diff-scope`, `open-set`, `large-files`.

9. `feat(codemap): add file-ops cleanup tools`
   - Add `delete-plan`, `file-move-plan`, `rename-plan`, `import-fix-plan`, `orphan-files`, `shim-check`, `barrel-check`, `case-check`, `text-check`, `asset-file-check`.

10. `feat(codemap): add patch/commit utilities`
   - Add `patch-preview`, `commit-files`, `workset`.

11. `feat(codemap): refine command grouping`
   - Align move/tauri command targets and registry/snapshot/fixtures coverage.

12. `docs(codemap): document refactor reports`
   - Update CLI help.
   - Update README examples.
   - Keep smoke commands documented.

Jeżeli potrzebujesz maksymalnie małych zmian, każdy punkt można podzielić na mniejsze PR-y
(np. jedno wejście na każdą nową komendę), ale powyższa kolejność to dobry punkt startu.
