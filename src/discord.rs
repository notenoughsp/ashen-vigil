//! Discord Rich Presence: shows friends on Discord what the player is doing. Talks to the
//! Discord app on this machine through its local IPC pipe, from a thread of its own, so a
//! missing or slow Discord never stalls the game. Needs a Discord application id.

use serde_json::json;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// The game's Discord application id. `ASHEN_DISCORD_ID` overrides it.
pub const APP_ID: &str = "1567613802726293625";

/// What the profile card shows.
#[derive(Clone, PartialEq, Debug)]
pub struct Presence {
    /// The top line, e.g. "Boss Rush, online".
    pub details: String,
    /// The second line, e.g. "Lord 4 of 12".
    pub state: String,
    /// Unix seconds when this activity began; Discord counts up from it.
    pub since: Option<i64>,
    /// Players in the party and how many fit.
    pub party: Option<(u32, u32)>,
}

pub struct Discord {
    tx: Option<Sender<Option<Presence>>>,
    last: Option<Option<Presence>>,
}

impl Discord {
    /// A stand-in that shows nothing.
    pub fn off() -> Discord {
        Discord { tx: None, last: None }
    }

    /// Starts the presence thread, or a silent stand-in without an application id.
    pub fn start() -> Discord {
        let id = std::env::var("ASHEN_DISCORD_ID").unwrap_or_else(|_| APP_ID.to_string());
        if id.trim().is_empty() || !cfg!(windows) {
            return Discord { tx: None, last: None };
        }
        let (tx, rx) = channel();
        std::thread::spawn(move || run(id.trim().to_string(), rx));
        Discord { tx: Some(tx), last: None }
    }

    /// The activity to show, or None to clear it. Only changes are passed on.
    pub fn set(&mut self, p: Option<Presence>) {
        if self.last.as_ref() == Some(&p) {
            return;
        }
        self.last = Some(p.clone());
        if let Some(tx) = &self.tx {
            let _ = tx.send(p);
        }
    }
}

/// The thread: connect, say hello, then send the newest activity at most every few seconds
/// (Discord allows five updates in twenty seconds). Reconnects when Discord comes and goes.
fn run(id: String, rx: Receiver<Option<Presence>>) {
    let mut latest: Option<Option<Presence>> = None;
    loop {
        let Some(mut pipe) = connect(&id) else {
            // Discord is not running: keep the newest wish and try again later.
            match rx.recv_timeout(Duration::from_secs(15)) {
                Ok(p) => latest = Some(p),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            while let Ok(p) = rx.try_recv() {
                latest = Some(p);
            }
            continue;
        };
        crate::log!("discord presence connected");
        let mut nonce = 0u64;
        loop {
            if latest.is_none() {
                match rx.recv() {
                    Ok(p) => latest = Some(p),
                    Err(_) => return,
                }
            }
            while let Ok(p) = rx.try_recv() {
                latest = Some(p);
            }
            let Some(p) = latest.take() else { continue };
            nonce += 1;
            if send_activity(&mut pipe, p.as_ref(), nonce).is_err() {
                crate::log!("discord presence lost the connection");
                latest = Some(p);
                break;
            }
            std::thread::sleep(Duration::from_secs(4));
        }
    }
}

#[cfg(windows)]
type Pipe = std::fs::File;
#[cfg(not(windows))]
type Pipe = std::io::Empty;

#[cfg(windows)]
fn connect(id: &str) -> Option<Pipe> {
    for k in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{}", k);
        if let Ok(mut f) = std::fs::OpenOptions::new().read(true).write(true).open(&path) {
            let hello = json!({ "v": 1, "client_id": id });
            if frame(&mut f, 0, &hello).is_err() {
                continue;
            }
            // Discord answers READY, or an error such as an unknown application id.
            match read_frame(&mut f) {
                Ok(answer) if String::from_utf8_lossy(&answer).contains("\"READY\"") => return Some(f),
                Ok(answer) => {
                    crate::log!("discord refused the presence: {}", String::from_utf8_lossy(&answer));
                    return None;
                }
                Err(_) => continue,
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn connect(_id: &str) -> Option<Pipe> {
    None
}

fn send_activity(pipe: &mut Pipe, p: Option<&Presence>, nonce: u64) -> std::io::Result<()> {
    let activity = p.map(|p| {
        let mut a = json!({
            "details": p.details,
            "assets": { "large_image": "icon", "large_text": "Ashen Vigil" },
        });
        if !p.state.is_empty() {
            a["state"] = json!(p.state);
        }
        if let Some(since) = p.since {
            a["timestamps"] = json!({ "start": since });
        }
        if let Some((n, max)) = p.party {
            a["party"] = json!({ "id": "ashen-vigil", "size": [n, max] });
        }
        a
    });
    let cmd = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": activity },
        "nonce": nonce.to_string(),
    });
    frame(pipe, 1, &cmd)?;
    read_frame(pipe).map(|_| ())
}

/// One IPC message: opcode and length, little endian, then the JSON.
fn frame(pipe: &mut Pipe, op: u32, body: &serde_json::Value) -> std::io::Result<()> {
    use std::io::Write;
    let data = body.to_string().into_bytes();
    let mut out = Vec::with_capacity(8 + data.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    pipe.write_all(&out)?;
    pipe.flush()
}

/// Reads and discards Discord's answer, so the pipe never fills up.
fn read_frame(pipe: &mut Pipe) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut head = [0u8; 8];
    pipe.read_exact(&mut head)?;
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    let mut body = vec![0u8; len.min(1 << 20)];
    pipe.read_exact(&mut body)?;
    Ok(body)
}

/// Says hello to Discord with `id` and returns its answer, for testing the pipe by hand.
#[cfg(all(test, windows))]
pub fn probe(id: &str) -> Result<String, String> {
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(r"\\.\pipe\discord-ipc-0").map_err(|e| format!("open: {}", e))?;
    frame(&mut f, 0, &json!({ "v": 1, "client_id": id })).map_err(|e| format!("write: {}", e))?;
    read_frame(&mut f).map(|b| String::from_utf8_lossy(&b).into_owned()).map_err(|e| format!("read: {}", e))
}
