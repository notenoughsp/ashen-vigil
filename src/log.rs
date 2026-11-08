//! A log of every session, so problems players run into can be looked at afterwards.
//!
//! Each start writes `%APPDATA%\AshenVigil\logs\<date>_<time>.log`; the newest ten are kept.
//! Panics land in the log with their location before the game closes. Without `init`
//! (in tests) logging does nothing.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Sessions kept in the log folder.
const KEEP: usize = 10;
/// A runaway log stops growing here.
const MAX_BYTES: u64 = 8 << 20;

struct Log {
    file: File,
    path: PathBuf,
    written: u64,
    start: Instant,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);

/// Writes one line, stamped with the time since start.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::log::write(&format!($($arg)*)) };
}

pub fn write(line: &str) {
    let Ok(mut guard) = LOG.lock() else { return };
    let Some(log) = guard.as_mut() else { return };
    if log.written > MAX_BYTES {
        return;
    }
    let t = log.start.elapsed().as_secs_f64();
    let text = format!("[{:02}:{:02}:{:06.3}] {}\n", (t / 3600.0) as u32, (t / 60.0) as u32 % 60, t % 60.0, line);
    if log.file.write_all(text.as_bytes()).is_ok() {
        log.written += text.len() as u64;
        // Flush right away: the line before a crash is the one that matters.
        let _ = log.file.flush();
    }
}

/// The folder for the player's logs.
pub fn log_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    return PathBuf::from(crate::save::ANDROID_DIR).join("logs");
    #[cfg(not(target_os = "android"))]
    match std::env::var_os("APPDATA") {
        Some(appdata) => PathBuf::from(appdata).join("AshenVigil").join("logs"),
        None => PathBuf::from("logs"),
    }
}

/// Opens a new log in `dir` (or at `file`, for the self-test) and catches panics into it.
pub fn init(dir: &Path, file: Option<PathBuf>) {
    let path = match file {
        Some(f) => f,
        None => {
            let _ = std::fs::create_dir_all(dir);
            prune(dir);
            dir.join(format!("{}.log", stamp()))
        }
    };
    let Ok(f) = File::create(&path) else { return };
    if let Ok(mut guard) = LOG.lock() {
        *guard = Some(Log { file: f, path: path.clone(), written: 0, start: Instant::now() });
    }
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write(&format!("CRASH: {}", info));
        write(&format!("backtrace:\n{}", std::backtrace::Backtrace::force_capture()));
        // A player in someone's vigil sends the crash to the host before going down.
        crate::net::upload_log("crash");
        default(info);
    }));
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    write(&format!("Ashen Vigil {} started, log {}", env!("CARGO_PKG_VERSION"), name));
}

/// Deletes all but the newest `KEEP - 1` logs, making room for this session's.
fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut logs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "log")).collect();
    // Names start with the date and time, so they sort by age.
    logs.sort();
    let extra = logs.len().saturating_sub(KEEP - 1);
    for p in logs.into_iter().take(extra) {
        let _ = std::fs::remove_file(p);
    }
}

/// "2026-11-03_19-06-42" in UTC, from the system clock.
fn stamp() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rem) = (secs / 86400, secs % 86400);
    let (y, m, d) = civil_from_days(days as i64);
    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}", y, m, d, rem / 3600, rem / 60 % 60, rem % 60)
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

/// Milliseconds since the log started, as a cheap unique id.
pub fn uptime_ms() -> u64 {
    LOG.lock().ok().and_then(|g| g.as_ref().map(|l| l.start.elapsed().as_millis() as u64)).unwrap_or(0)
}

/// This session's log so far, to send to a host.
pub fn current_text() -> Option<String> {
    let path = LOG.lock().ok()?.as_ref()?.path.clone();
    std::fs::read_to_string(path).ok()
}

/// Players' logs a host received, the newest twenty kept.
const KEEP_RECEIVED: usize = 20;

/// Stores a log a player sent; returns where it went.
pub fn store_received(dir: &Path, player: &str, reason: &str, text: &str) -> Option<PathBuf> {
    let dir = dir.join("players");
    std::fs::create_dir_all(&dir).ok()?;
    let safe: String = player.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).take(16).collect();
    let reason: String = reason.chars().filter(|c| c.is_ascii_alphanumeric()).take(12).collect();
    let path = dir.join(format!("{}_{}_{}.log", stamp(), safe, reason));
    std::fs::write(&path, text).ok()?;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut logs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "log")).collect();
        logs.sort();
        let extra = logs.len().saturating_sub(KEEP_RECEIVED);
        for p in logs.into_iter().take(extra) {
            let _ = std::fs::remove_file(p);
        }
    }
    Some(path)
}

#[cfg(test)]
pub fn date_of(days: i64) -> (i64, u32, u32) {
    civil_from_days(days)
}
