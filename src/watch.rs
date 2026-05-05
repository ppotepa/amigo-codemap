use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::cli::Options;
use crate::{output, scan};

const DEBOUNCE: Duration = Duration::from_millis(300);
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

    let initial = scan::scan_project(&options)?;
    output::write_codemap(&options, &initial)?;
    println!(
        "watching {} -> {}",
        options.root.display(),
        options.out.display()
    );

    let mut pending = false;
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
                pending = true;
                last_event = Instant::now();
            }
            Ok(Err(error)) => return Err(anyhow!("watch error: {error}")),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("watcher disconnected"));
            }
        }

        if pending && last_event.elapsed() >= DEBOUNCE && last_write.elapsed() >= MIN_WRITE_INTERVAL
        {
            let map = scan::scan_project(&options)?;
            let wrote = output::write_codemap(&options, &map)?;
            if wrote {
                println!("updated {}", options.out.display());
            }
            pending = false;
            last_write = Instant::now();
        }
    }
}

fn should_ignore_event(options: &Options, path: &std::path::Path) -> bool {
    path.starts_with(options.root.join(".git"))
        || path.starts_with(options.root.join(".amigo"))
        || path.starts_with(options.root.join("target"))
        || path.starts_with(options.root.join("node_modules"))
}
