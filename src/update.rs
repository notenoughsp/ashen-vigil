//! Checks GitHub for a newer release and, when the player asks, puts it in place of the
//! running executable. Talks to GitHub through Windows' own curl.exe on a thread of its own,
//! so the game needs no web library and never waits on the network.
//!
//! Updating: the new exe is downloaded next to the old one as `<exe>.new` and its size checked;
//! then the running exe is renamed to `<exe>.old` (Windows allows renaming a running program),
//! the new one takes its name, and the game restarts. The next start removes `<exe>.old`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const LATEST: &str = "https://api.github.com/repos/notenoughsp/ashen-vigil/releases/latest";
/// The release assets that are the game: with the recorded songs, and with synthesized music only.
const ASSET_SONGS: &str = "AshenVigil.exe";
const ASSET_SYNTH: &str = "AshenVigil-Synth.exe";

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    /// Not checked (turned off, a development build, or no answer yet).
    Idle,
    /// A newer version is out.
    Available {
        version: String,
    },
    Downloading {
        version: String,
    },
    /// The new version is in place; a restart starts it.
    Ready {
        version: String,
    },
    Failed(String),
}

/// One downloadable build of a release.
#[derive(Clone)]
pub struct Asset {
    pub url: String,
    pub size: u64,
}

pub struct Found {
    pub version: String,
    /// The build with the recorded songs, and the one with synthesized music only.
    pub songs: Option<Asset>,
    pub synth: Option<Asset>,
}

pub struct Updater {
    state: Arc<Mutex<State>>,
    found: Arc<Mutex<Option<Found>>>,
    /// The build last asked for (true: with songs), for a retry.
    chose_songs: Arc<Mutex<bool>>,
}

impl Updater {
    /// A stand-in that never checks.
    pub fn off() -> Updater {
        Updater { state: Arc::new(Mutex::new(State::Idle)), found: Arc::new(Mutex::new(None)), chose_songs: Arc::new(Mutex::new(cfg!(feature = "songs"))) }
    }

    /// Shows a newer version without asking anyone, for the self-test's screenshots.
    pub fn pretend(version: &str) -> Updater {
        let up = Updater::off();
        *up.state.lock().unwrap() = State::Available { version: version.to_string() };
        let fake = |size| Some(Asset { url: String::new(), size });
        *up.found.lock().unwrap() = Some(Found { version: version.to_string(), songs: fake(57_135_616), synth: fake(4_936_704) });
        up
    }

    /// Cleans up after the last update and asks GitHub for the newest release.
    pub fn start() -> Updater {
        let up = Updater::off();
        let Some(exe) = own_exe() else { return up };
        let _ = std::fs::remove_file(with_ext(&exe, "old"));
        let _ = std::fs::remove_file(with_ext(&exe, "new"));
        let (state, found) = (up.state.clone(), up.found.clone());
        std::thread::spawn(move || match latest() {
            Ok(f) if newer(&f.version, crate::net::VERSION) => {
                crate::log!("update: {} is out (this is {})", f.version, crate::net::VERSION);
                *state.lock().unwrap() = State::Available { version: f.version.clone() };
                *found.lock().unwrap() = Some(f);
            }
            Ok(f) => crate::log!("update: {} is the newest, this is {}", f.version, crate::net::VERSION),
            Err(e) => crate::log!("update check failed: {}", e),
        });
        up
    }

    pub fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    /// The sizes in bytes of the two builds on offer: with songs, and synthesized only.
    pub fn choices(&self) -> (Option<u64>, Option<u64>) {
        self.found.lock().unwrap().as_ref().map_or((None, None), |f| (f.songs.as_ref().map(|a| a.size), f.synth.as_ref().map(|a| a.size)))
    }

    /// Tries the build asked for last again.
    pub fn retry(&self) {
        let songs = *self.chose_songs.lock().unwrap();
        self.install(songs);
    }

    /// Downloads the newer version, with the recorded songs or without, and puts it in place,
    /// on a thread. If that build is missing from the release, the other one is taken.
    pub fn install(&self, songs: bool) {
        let Some(exe) = own_exe() else { return };
        let picked = self.found.lock().unwrap().as_ref().and_then(|f| {
            let (want, other) = if songs { (&f.songs, &f.synth) } else { (&f.synth, &f.songs) };
            want.clone().or_else(|| other.clone()).map(|a| (f.version.clone(), a.url, a.size))
        });
        let Some((version, url, size)) = picked else { return };
        if !matches!(self.state(), State::Available { .. } | State::Failed(_)) {
            return;
        }
        *self.chose_songs.lock().unwrap() = songs;
        crate::log!("update: downloading {} {}", version, if songs { "with songs" } else { "with synthesized music" });
        *self.state.lock().unwrap() = State::Downloading { version: version.clone() };
        let state = self.state.clone();
        std::thread::spawn(move || {
            let next = match install(&exe, &url, size) {
                Ok(()) => {
                    crate::log!("update: {} is in place", version);
                    State::Ready { version }
                }
                Err(e) => {
                    crate::log!("update failed: {}", e);
                    State::Failed(e)
                }
            };
            *state.lock().unwrap() = next;
        });
    }

    /// Starts the updated game and ends this one.
    pub fn restart(&self) {
        if !matches!(self.state(), State::Ready { .. }) {
            return;
        }
        if let Some(exe) = own_exe() {
            crate::log!("update: restarting");
            if std::process::Command::new(&exe).spawn().is_ok() {
                std::process::exit(0);
            }
        }
    }
}

/// The running game's executable, if it may be updated: not a development build.
fn own_exe() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let dev = exe.components().any(|c| c.as_os_str().eq_ignore_ascii_case("target"));
    (!dev).then_some(exe)
}

fn with_ext(exe: &std::path::Path, ext: &str) -> PathBuf {
    let mut s = exe.as_os_str().to_owned();
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

/// curl.exe without a console window flashing up.
fn curl() -> std::process::Command {
    let mut c = std::process::Command::new("curl.exe");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c.args(["-s", "-f", "-L", "-A", "AshenVigil-updater"]);
    c
}

pub(crate) fn latest() -> Result<Found, String> {
    let out = curl().args(["-m", "15", "-H", "Accept: application/vnd.github+json", LATEST]).output().map_err(|e| format!("curl: {}", e))?;
    if !out.status.success() {
        return Err(format!("GitHub answered with curl status {:?}", out.status.code()));
    }
    parse_latest(&String::from_utf8_lossy(&out.stdout))
}

/// Reads GitHub's answer about the latest release.
pub fn parse_latest(json: &str) -> Result<Found, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("bad answer: {}", e))?;
    let tag = v["tag_name"].as_str().ok_or("no tag in the answer")?;
    let asset = |name: &str| {
        let a = v["assets"].as_array()?.iter().find(|x| x["name"] == name)?;
        Some(Asset { url: a["browser_download_url"].as_str()?.to_string(), size: a["size"].as_u64()? })
    };
    let (songs, synth) = (asset(ASSET_SONGS), asset(ASSET_SYNTH));
    if songs.is_none() && synth.is_none() {
        return Err("the release has no AshenVigil.exe".into());
    }
    Ok(Found { version: tag.trim_start_matches('v').to_string(), songs, synth })
}

/// Whether version `a` ("0.2.3") is newer than `b`.
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |s: &str| s.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<u32>>();
    parts(a) > parts(b)
}

pub(crate) fn install(exe: &std::path::Path, url: &str, size: u64) -> Result<(), String> {
    let new = with_ext(exe, "new");
    let old = with_ext(exe, "old");
    let status = curl().args(["-m", "600", "-o"]).arg(&new).arg(url).status().map_err(|e| format!("curl: {}", e))?;
    let got = std::fs::metadata(&new).map(|m| m.len()).unwrap_or(0);
    if !status.success() || got != size {
        let _ = std::fs::remove_file(&new);
        return Err(format!("the download broke off ({} of {} bytes)", got, size));
    }
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old).map_err(|e| format!("could not move the old game aside: {}", e))?;
    if let Err(e) = std::fs::rename(&new, exe) {
        // Put the old game back so nothing is lost.
        let _ = std::fs::rename(&old, exe);
        return Err(format!("could not put the new game in place: {}", e));
    }
    Ok(())
}
