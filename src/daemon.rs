use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::cli::Options;
use crate::daemon_protocol::{
    DEFAULT_DAEMON_ADDR, DaemonMapOptions, DaemonRequest, DaemonResponse, DaemonStatus,
};
use crate::model::CodeMap;
use crate::{output, scan, snapshot_store};

#[derive(Debug)]
struct WorkspaceState {
    root: PathBuf,
    out: PathBuf,
    level: u8,
    pretty: bool,
    ai: bool,
    generated_at_unix_ms: u64,
    map: CodeMap,
}

// @codemap P1 codemap-daemon-runtime
pub fn run_from_env_args() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    let mut command = "run".to_string();
    let mut root = std::env::current_dir()?;
    let mut addr = DEFAULT_DAEMON_ADDR.to_string();
    let mut level = 2u8;
    let mut ai = false;
    let mut pretty = false;
    let mut out: Option<PathBuf> = None;

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "run" | "start" | "status" | "shutdown" => {
                command = args[index].clone();
            }
            "--root" => {
                index += 1;
                root = PathBuf::from(required_arg(&args, index, "--root")?);
            }
            "--addr" => {
                index += 1;
                addr = required_arg(&args, index, "--addr")?.to_string();
            }
            "--level" => {
                index += 1;
                level = required_arg(&args, index, "--level")?.parse::<u8>()?;
                if level > 3 {
                    bail!("--level must be 0, 1, 2, or 3");
                }
            }
            "--ai" => ai = true,
            "--pretty" => pretty = true,
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(required_arg(&args, index, "--out")?));
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            value => bail!("unknown amigo-codemapd argument `{value}`"),
        }
        index += 1;
    }

    let addr = addr.parse::<SocketAddr>()?;

    match command.as_str() {
        "run" | "start" => run_daemon(root, out, addr, level, pretty, ai),
        "status" => print_remote_status(addr),
        "shutdown" => shutdown_remote(addr),
        _ => unreachable!(),
    }
}

fn print_help() {
    println!(
        "amigo-codemapd\n\ncommands:\n  run|start       start local codemap daemon\n  status          query running daemon\n  shutdown        stop running daemon\n\noptions:\n  --root <path>   workspace root, default current directory\n  --addr <addr>   default {DEFAULT_DAEMON_ADDR}\n  --level <0-3>   default 2\n  --ai\n  --pretty\n  --out <path>    default <root>\\.amigo\\codemap.json"
    );
}

fn print_remote_status(addr: SocketAddr) -> Result<()> {
    match send_request(addr, DaemonRequest::Status)? {
        DaemonResponse::Status { status } => {
            println!("codemap daemon status");
            println!("  root: {}", status.root);
            println!("  out: {}", status.out);
            println!("  level: {}", status.level);
            println!("  ai: {}", status.ai);
            println!("  dirty: {}", status.dirty);
            println!("  generated_at_unix_ms: {}", status.generated_at_unix_ms);
            println!("  files: {}", status.files);
            println!("  packages: {}", status.packages);
            println!("  symbols: {}", status.symbols);
            println!("  dependencies: {}", status.dependencies);
            println!("  areas: {}", status.areas);
            Ok(())
        }
        DaemonResponse::Error { message } => bail!("{message}"),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

fn shutdown_remote(addr: SocketAddr) -> Result<()> {
    match send_request(addr, DaemonRequest::Shutdown)? {
        DaemonResponse::Ok => {
            println!("codemap daemon shutdown requested");
            Ok(())
        }
        DaemonResponse::Error { message } => bail!("{message}"),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

fn send_request(addr: SocketAddr, request: DaemonRequest) -> Result<DaemonResponse> {
    let mut stream = TcpStream::connect(addr)?;
    serde_json::to_writer(&mut stream, &request)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    if response.trim().is_empty() {
        bail!("empty response from daemon");
    }

    Ok(serde_json::from_str::<DaemonResponse>(&response)?)
}

fn run_daemon(
    root: PathBuf,
    out: Option<PathBuf>,
    addr: SocketAddr,
    level: u8,
    pretty: bool,
    ai: bool,
) -> Result<()> {
    let root = root.canonicalize().unwrap_or(root);
    let out = out.unwrap_or_else(|| root.join(".amigo").join("codemap.json"));

    let options = make_options(&root, &out, level, pretty, ai);
    let map = scan::scan_project(&options)?;
    output::write_codemap(&options, &map)?;
    snapshot_store::write_snapshot(&options, &map)?;

    let state = Arc::new(Mutex::new(WorkspaceState {
        root: root.clone(),
        out: out.clone(),
        level,
        pretty,
        ai,
        generated_at_unix_ms: now_ms(),
        map,
    }));

    let dirty = Arc::new(AtomicBool::new(false));
    start_watcher(root.clone(), dirty.clone())?;

    let listener = TcpListener::bind(addr)?;
    println!("amigo-codemapd listening on {addr}");
    println!("  root: {}", root.display());
    println!("  out: {}", out.display());

    for incoming in listener.incoming() {
        let stream = incoming?;
        let should_stop = handle_connection(stream, &state, &dirty)?;
        if should_stop {
            break;
        }
    }

    println!("amigo-codemapd stopped");
    Ok(())
}

fn handle_connection(
    stream: TcpStream,
    state: &Arc<Mutex<WorkspaceState>>,
    dirty: &Arc<AtomicBool>,
) -> Result<bool> {
    let mut reader = BufReader::new(stream);
    let mut input = String::new();
    reader.read_line(&mut input)?;

    let request = serde_json::from_str::<DaemonRequest>(&input);
    let response = match request {
        Ok(DaemonRequest::Status) => DaemonResponse::Status {
            status: current_status(state, dirty)?,
        },
        Ok(DaemonRequest::GetMap { options }) => match ensure_fresh_map(state, dirty, &options) {
            Ok(stale_was_refreshed) => {
                let map = state
                    .lock()
                    .map_err(|_| anyhow!("daemon state lock poisoned"))?
                    .map
                    .clone();
                DaemonResponse::Map {
                    map,
                    stale_was_refreshed,
                }
            }
            Err(error) => DaemonResponse::Error {
                message: error.to_string(),
            },
        },
        Ok(DaemonRequest::Refresh { options }) => match refresh_map(state, dirty, &options) {
            Ok(wrote_output) => {
                let generated_at_unix_ms = state
                    .lock()
                    .map_err(|_| anyhow!("daemon state lock poisoned"))?
                    .generated_at_unix_ms;
                DaemonResponse::Refreshed {
                    wrote_output,
                    generated_at_unix_ms,
                }
            }
            Err(error) => DaemonResponse::Error {
                message: error.to_string(),
            },
        },
        Ok(DaemonRequest::Shutdown) => {
            write_response(reader.get_mut(), &DaemonResponse::Ok)?;
            return Ok(true);
        }
        Err(error) => DaemonResponse::Error {
            message: error.to_string(),
        },
    };

    write_response(reader.get_mut(), &response)?;
    Ok(false)
}

fn write_response(stream: &mut TcpStream, response: &DaemonResponse) -> Result<()> {
    serde_json::to_writer(&mut *stream, response)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn ensure_fresh_map(
    state: &Arc<Mutex<WorkspaceState>>,
    dirty: &Arc<AtomicBool>,
    options: &DaemonMapOptions,
) -> Result<bool> {
    {
        let state = state
            .lock()
            .map_err(|_| anyhow!("daemon state lock poisoned"))?;
        ensure_same_root(&state.root, &options.root)?;

        if !dirty.load(Ordering::Relaxed) && state.level == options.level && state.ai == options.ai
        {
            return Ok(false);
        }
    }

    refresh_map(state, dirty, options)?;
    Ok(true)
}

fn refresh_map(
    state: &Arc<Mutex<WorkspaceState>>,
    dirty: &Arc<AtomicBool>,
    options: &DaemonMapOptions,
) -> Result<bool> {
    let root = PathBuf::from(&options.root)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&options.root));
    let out = PathBuf::from(&options.out);

    {
        let state = state
            .lock()
            .map_err(|_| anyhow!("daemon state lock poisoned"))?;
        ensure_same_root(&state.root, &root.to_string_lossy())?;
    }

    let scan_options = make_options(&root, &out, options.level, options.pretty, options.ai);
    let map = scan::scan_project(&scan_options)?;
    let wrote_output = output::write_codemap(&scan_options, &map)?;
    snapshot_store::write_snapshot(&scan_options, &map)?;

    {
        let mut state = state
            .lock()
            .map_err(|_| anyhow!("daemon state lock poisoned"))?;
        state.out = out;
        state.level = options.level;
        state.pretty = options.pretty;
        state.ai = options.ai;
        state.generated_at_unix_ms = now_ms();
        state.map = map;
    }

    dirty.store(false, Ordering::Relaxed);
    Ok(wrote_output)
}

fn current_status(
    state: &Arc<Mutex<WorkspaceState>>,
    dirty: &Arc<AtomicBool>,
) -> Result<DaemonStatus> {
    let state = state
        .lock()
        .map_err(|_| anyhow!("daemon state lock poisoned"))?;

    Ok(DaemonStatus {
        root: state.root.to_string_lossy().to_string(),
        out: state.out.to_string_lossy().to_string(),
        level: state.level,
        ai: state.ai,
        dirty: dirty.load(Ordering::Relaxed),
        files: state.map.files.len(),
        packages: state.map.packages.len(),
        symbols: state.map.symbols.len(),
        dependencies: state.map.dependencies.len(),
        areas: state.map.areas.len(),
        generated_at_unix_ms: state.generated_at_unix_ms,
    })
}

fn start_watcher(root: PathBuf, dirty: Arc<AtomicBool>) -> Result<()> {
    thread::Builder::new()
        .name("amigo-codemapd-watch".to_string())
        .spawn(move || {
            let dirty_for_callback = dirty.clone();
            let watch_root = root.clone();

            let watcher_result = RecommendedWatcher::new(
                move |result: notify::Result<Event>| {
                    if let Ok(event) = result {
                        if event.paths.iter().any(|path| should_ignore_event(path)) {
                            return;
                        }
                        let _ = snapshot_store::mark_dirty(&watch_root);
                        dirty_for_callback.store(true, Ordering::Relaxed);
                    }
                },
                Config::default(),
            );

            let Ok(mut watcher) = watcher_result else {
                dirty.store(true, Ordering::Relaxed);
                return;
            };

            if watcher.watch(&root, RecursiveMode::Recursive).is_err() {
                dirty.store(true, Ordering::Relaxed);
                return;
            }

            loop {
                thread::park();
            }
        })?;

    Ok(())
}

fn should_ignore_event(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git" | ".amigo" | "target" | "node_modules" | "dist" | "build" | "coverage"
        )
    }) || path.extension().is_some_and(|ext| {
        matches!(
            ext.to_string_lossy().as_ref(),
            "tmp" | "swp" | "lock" | "log"
        )
    })
}

fn ensure_same_root(expected: &Path, received: &str) -> Result<()> {
    let received = PathBuf::from(received)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(received));
    let expected = expected
        .canonicalize()
        .unwrap_or_else(|_| expected.to_path_buf());

    if expected != received {
        bail!(
            "daemon root mismatch; daemon={}, request={}",
            expected.display(),
            received.display()
        );
    }

    Ok(())
}

fn make_options(root: &Path, out: &Path, level: u8, pretty: bool, ai: bool) -> Options {
    Options {
        root: root.to_path_buf(),
        out: out.to_path_buf(),
        level,
        pretty,
        ai,
        query: None,
        start_line: None,
        end_line: None,
        group: None,
        lines: false,
        line_range: None,
        limit: 80,
        min_score: 0,
        report: false,
        file_lines: 450,
        verify_args: Vec::new(),
        changed_only: false,
        patterns: Vec::new(),
        file: None,
        from: None,
        yaml: None,
        yaml_op: "replace_range".to_string(),
        by: None,
        to: None,
        symbol: None,
        task: None,
        from_impact: None,
        radius: 32,
        context_radius: 3,
        top: 20,
        with_split_hints: false,
        save: false,
        status: false,
        write: false,
        strict: false,
        backup: false,
        stop_on_error: false,
        run: false,
        why: false,
        metadata: false,
        json: false,
        raw: false,
        no_verbose: false,
        quiet: false,
        no_cache: false,
        compact: false,
        hide_generated: false,
        include_tests: false,
        include_generated: false,
        warnings: false,
    }
}

fn required_arg<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| anyhow!("{name} requires a value"))
}

fn now_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
