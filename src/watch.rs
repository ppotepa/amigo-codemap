use std::sync::mpsc;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::cli::Options;
use crate::{incremental, snapshot_store};

const DEBOUNCE: Duration = Duration::from_millis(700);
const MIN_WRITE_INTERVAL: Duration = Duration::from_secs(1);

pub fn watch_project(options: Options) -> Result<()> {
    let (sender, receiver) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let _ = sender.send(result);
        },
        Config::default(),
    )?;
    watcher.watch(&options.root, RecursiveMode::Recursive)?;

    let initial_started = Instant::now();
    let mut index = incremental::WorkspaceIndex::from_full_scan(&options)?;
    incremental::write_outputs(&options, &index.map)?;
    println!(
        "watching {} -> {} and {} ({:?})",
        options.root.display(),
        options.out.display(),
        snapshot_store::snapshot_path(&options.root).display(),
        initial_started.elapsed()
    );

    let mut pending = BTreeSet::<std::path::PathBuf>::new();
    let mut last_event = Instant::now();
    let mut last_write = Instant::now() - MIN_WRITE_INTERVAL;

    loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                if event
                    .paths
                    .iter()
                    .any(|path| should_ignore_event(&options, path))
                {
                    continue;
                }
                let _ = snapshot_store::mark_dirty(&options.root);
                pending.extend(event.paths.into_iter());
                last_event = Instant::now();
            }
            Ok(Err(error)) => return Err(anyhow!("watch error: {error}")),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("watcher disconnected"));
            }
        }

        if !pending.is_empty()
            && last_event.elapsed() >= DEBOUNCE
            && last_write.elapsed() >= MIN_WRITE_INTERVAL
        {
            let started = Instant::now();
            let touched = pending.iter().cloned().collect::<Vec<_>>();
            index.refresh_touched(&options, &touched)?;
            let wrote = incremental::write_outputs(&options, &index.map)?;
            if wrote {
                println!(
                    "updated {} and {} in {:?}",
                    options.out.display(),
                    snapshot_store::snapshot_path(&options.root).display(),
                    started.elapsed()
                );
            } else {
                println!(
                    "refreshed snapshot {} in {:?}",
                    snapshot_store::snapshot_path(&options.root).display(),
                    started.elapsed()
                );
            }
            pending.clear();
            last_write = Instant::now();
        }
    }
}

fn should_ignore_event(options: &Options, path: &std::path::Path) -> bool {
    path.starts_with(options.root.join(".git"))
        || path.starts_with(options.root.join(".amigo"))
        || path.starts_with(options.root.join("target"))
        || path.starts_with(options.root.join("node_modules"))
        || path.starts_with(options.root.join("dist"))
        || path.starts_with(options.root.join("build"))
        || path.starts_with(options.root.join("coverage"))
        || path.extension().is_some_and(|ext| {
            matches!(
                ext.to_string_lossy().as_ref(),
                "tmp" | "swp" | "lock" | "log"
            )
        })
}
