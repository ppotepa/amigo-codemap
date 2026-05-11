pub fn print_help() {
    println!(
        "amigo-codemap

Usage:
  amigo-codemap <command> [options] [query]

Core commands:
  refresh              rebuild the snapshot
  status               show daemon/snapshot status
  watch                watch workspace changes
  scan                 run a scan once
  brief                compact summary
  changes              show changed files
  files                list indexed files
  symbols              list symbols
  trace                trace symbols and references
  trace-field          trace chained field access
  where                locate symbols
  signature            show extracted signature
  slice                show a symbol slice
  range-for-symbol     show raw range and raw ops hints
  verify-plan          validate a change plan
  verify               run verification checks
  fallout              summarize failing paths
  open-set             rank files for a task
  workset              inspect workset

Ops commands:
  ops-preview          preview raw operations
  ops-check            validate raw operations
  ops-apply            apply raw operations
  ops-summary          summarize raw operations
  ops-skeleton         generate raw op skeleton
  ops-schema           print raw op schema
  ops-split            split operations into files
  ops-verify           verify raw operation results

Navigation / planning:
  change-plan
  explain-file
  neighbors
  callsite-candidates
  api-surface
  component-graph
  tauri-graph
  todo-index
  risk-index
  smells
  command-map
  anchors
  anchor-check
  taxonomy
  impact
  move-plan
  dup
  service-shape
  registry-check
  metadata-audit
  descriptor-skeleton
  commit-plan
  commit-summary
  append-plan
  copy-plan
  diff-scope
  delete-plan
  file-move-plan
  rename-plan
  import-fix-plan
  barrel-check
  orphan-files
  shim-check
  large-files
  asset-file-check
  case-check
  text-check
  patch-preview
  patch-check
  patch-apply
  commit-files

Flags:
  --root <path>              workspace root
  --out <path>               output directory
  --level <0-3>              scan depth
  --pretty                   pretty output
  --ai                       AI-oriented output
  --query <text>             explicit query
  --group <name>             grouping key
  --lines                    line-aware output
  --limit <n>                result limit
  --min-score <n>            minimum score
  --report                   emit report
  --report-file <path>       write report file
  --changed-only             changed files only
  --file <path>              target file
  --from <path>              source path
  --to <path>                destination path
  --symbol <name>            target symbol
  --task <name>              task hint
  --timings                  print timings
  --progress                 print progress
  --diagnostics              print diagnostics
  --slow-file-threshold-ms <n>  slow file threshold
  --max-file-size <bytes>    max file size
  --max-files <n>            max files
  --status                   show status
  --write                    write changes
  --strict                   strict validation
  --backup                   create backups
  --stop-on-error            stop on first error
  --run                      run generated action
  --why                      explain ranking
  --metadata                 print metadata
  --json                     JSON output
  --raw                      raw ops mode
  --no-verbose               suppress verbose output
  --quiet                    quiet mode
  --no-cache                 bypass cache
  --compact                  compact output
  --hide-generated           hide generated files
  --include-tests            include tests
  --include-generated        include generated files
  --warnings                 print warnings
  --expect-present <query>   expect symbol/text to exist
  --expect-absent <query>    expect symbol/text to be absent
  --daemon <mode>            daemon mode: auto|require|disabled
  --no-daemon                disable daemon usage
  --help, -h                 show help
"
    );
}
