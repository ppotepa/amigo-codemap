use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use anyhow::{Result, anyhow};

use crate::cli::DaemonMode;
use crate::cli::Options;
use crate::daemon_protocol::{
    DEFAULT_DAEMON_ADDR, DaemonMapOptions, DaemonRequest, DaemonResponse,
};
use crate::model::CodeMap;

// @codemap P1 codemap-daemon-client-local-snapshot
pub fn try_load_map(options: &Options) -> Result<Option<CodeMap>> {
    if options.no_cache || daemon_is_disabled(options) {
        return Ok(None);
    }

    let request = DaemonRequest::GetMap {
        options: map_options(options),
    };

    match send_request(request) {
        Ok(DaemonResponse::Map { map, .. }) => Ok(Some(map)),
        Ok(DaemonResponse::Error { message }) => {
            if verbose_daemon_client() {
                eprintln!("codemap daemon returned error; using local snapshot: {message}");
            }
            Ok(None)
        }
        Ok(other) => {
            if verbose_daemon_client() {
                eprintln!("unexpected codemap daemon response {other:?}; using local snapshot");
            }
            Ok(None)
        }
        Err(error) => match options.daemon_mode {
            DaemonMode::Require => Err(error),
            _ => {
                if verbose_daemon_client() {
                    eprintln!("codemap daemon unavailable; using local snapshot: {error}");
                }
                Ok(None)
            }
        },
    }
}

fn daemon_is_disabled(options: &Options) -> bool {
    matches!(options.daemon_mode, DaemonMode::Disabled)
        || std::env::var("AMIGO_CODEMAP_NO_DAEMON")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn verbose_daemon_client() -> bool {
    std::env::var("AMIGO_CODEMAP_DAEMON_VERBOSE")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn map_options(options: &Options) -> DaemonMapOptions {
    DaemonMapOptions {
        root: options.root.to_string_lossy().to_string(),
        out: options.out.to_string_lossy().to_string(),
        level: options.level,
        pretty: options.pretty,
        ai: options.ai,
    }
}

pub fn send_request(request: DaemonRequest) -> Result<DaemonResponse> {
    let addr = daemon_addr()?;
    let connect_timeout = Duration::from_millis(env_timeout_ms(
        "AMIGO_CODEMAP_DAEMON_CONNECT_TIMEOUT_MS",
        80,
    ));
    let io_timeout =
        Duration::from_millis(env_timeout_ms("AMIGO_CODEMAP_DAEMON_IO_TIMEOUT_MS", 1500));

    let mut stream = TcpStream::connect_timeout(&addr, connect_timeout)?;
    stream.set_read_timeout(Some(io_timeout))?;
    stream.set_write_timeout(Some(io_timeout))?;

    serde_json::to_writer(&mut stream, &request)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    if response.trim().is_empty() {
        return Err(anyhow!("empty response from codemap daemon"));
    }

    Ok(serde_json::from_str::<DaemonResponse>(&response)?)
}

fn daemon_addr() -> Result<SocketAddr> {
    let value = std::env::var("AMIGO_CODEMAP_DAEMON_ADDR")
        .unwrap_or_else(|_| DEFAULT_DAEMON_ADDR.to_string());
    Ok(value.parse::<SocketAddr>()?)
}

fn env_timeout_ms(name: &str, default_value: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default_value)
}
