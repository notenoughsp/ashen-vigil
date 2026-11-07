//! UDP networking: host-authoritative co-op with up to four players.
//!
//! Clients send their input (and their own predicted position) 60 times a second; the host
//! simulates the world and sends each client its own snapshot, split into fragments, 30 times
//! a second. Nothing needs to be reliable: inputs carry complete state, and snapshots are
//! coded against the newest one the client acknowledged in its input, so a lost packet is
//! simply superseded by the next one.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};

pub const DEFAULT_PORT: u16 = 7777;

/// The port to host on and join by default: 7777, or ASHEN_PORT (tests run beside a real game).
pub fn game_port() -> u16 {
    std::env::var("ASHEN_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_PORT)
}
// The last byte is the protocol version; different builds refuse to talk to each other.
const MAGIC: [u8; 3] = [0xA5, 0x1C, 18];
/// The highest field number, see `world::ALL_BIOMES`.
const ALL_FIELDS_MAX: u8 = crate::world::ALL_BIOMES.len() as u8 - 1;
/// Shown in menus, so players can tell whether they run the same build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const FRAG_PAYLOAD: usize = 1100;
pub const TIMEOUT: f64 = 6.0;
pub const SNAPSHOT_RATE: f32 = 30.0;
/// While the world stands still for a level-up, only choices and the countdown change.
pub const PAUSED_SNAPSHOT_RATE: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum PType {
    Hello = 1,
    Welcome = 2,
    Lobby = 3,
    Input = 4,
    Snap = 5,
    Ping = 6,
    Pong = 7,
    Leave = 8,
    Refused = 9,
    /// A fragment of a player's log, sent to the host after a vigil or a crash.
    Log = 10,
}

impl PType {
    fn from(v: u8) -> Option<PType> {
        use PType::*;
        Some(match v {
            1 => Hello,
            2 => Welcome,
            3 => Lobby,
            4 => Input,
            5 => Snap,
            6 => Ping,
            7 => Pong,
            8 => Leave,
            9 => Refused,
            10 => Log,
            _ => return None,
        })
    }
}

#[derive(Default)]
pub struct Writer(pub Vec<u8>);

impl Writer {
    pub fn packet(t: PType) -> Writer {
        let mut w = Writer(Vec::with_capacity(256));
        w.0.extend_from_slice(&MAGIC);
        w.u8(t as u8);
        w
    }
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub fn bool(&mut self, v: bool) {
        self.0.push(v as u8);
    }
    pub fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i16(&mut self, v: i16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// 7 bits per byte; small numbers take one byte.
    pub fn varint(&mut self, mut v: u32) {
        while v >= 0x80 {
            self.0.push(v as u8 | 0x80);
            v >>= 7;
        }
        self.0.push(v as u8);
    }
    pub fn str(&mut self, s: &str) {
        let b = s.as_bytes();
        let n = b.len().min(255);
        self.u8(n as u8);
        self.0.extend_from_slice(&b[..n]);
    }
}

pub struct Reader<'a> {
    b: &'a [u8],
    i: usize,
    pub ok: bool,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Reader<'a> {
        Reader { b, i: 0, ok: true }
    }
    fn take(&mut self, n: usize) -> &'a [u8] {
        if self.i + n > self.b.len() {
            self.ok = false;
            self.i = self.b.len();
            return &[];
        }
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        s
    }
    pub fn u8(&mut self) -> u8 {
        self.take(1).first().copied().unwrap_or(0)
    }
    pub fn bool(&mut self) -> bool {
        self.u8() != 0
    }
    pub fn u16(&mut self) -> u16 {
        let s = self.take(2);
        if s.len() < 2 {
            0
        } else {
            u16::from_le_bytes([s[0], s[1]])
        }
    }
    pub fn i16(&mut self) -> i16 {
        self.u16() as i16
    }
    pub fn u32(&mut self) -> u32 {
        let s = self.take(4);
        if s.len() < 4 {
            0
        } else {
            u32::from_le_bytes([s[0], s[1], s[2], s[3]])
        }
    }
    pub fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }
    pub fn u64(&mut self) -> u64 {
        let s = self.take(8);
        if s.len() < 8 {
            return 0;
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        u64::from_le_bytes(a)
    }
    pub fn f64(&mut self) -> f64 {
        let s = self.take(8);
        if s.len() < 8 {
            return 0.0;
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        f64::from_le_bytes(a)
    }
    pub fn varint(&mut self) -> u32 {
        let mut v = 0u32;
        for shift in (0..35).step_by(7) {
            let b = self.u8();
            v |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                return v;
            }
        }
        self.ok = false;
        v
    }
    pub fn str(&mut self) -> String {
        let n = self.u8() as usize;
        String::from_utf8_lossy(self.take(n)).into_owned()
    }
    pub fn bytes(&mut self, n: usize) -> &'a [u8] {
        self.take(n)
    }
    pub fn rest(&mut self) -> &'a [u8] {
        let s = &self.b[self.i..];
        self.i = self.b.len();
        s
    }
}

/// Traffic counters shown in the overlay.
#[derive(Default)]
pub struct NetStats {
    pub sent_total: u64,
    pub recv_total: u64,
    pub sent_rate: f32,
    pub recv_rate: f32,
    pub pkts_sent_rate: f32,
    pub pkts_recv_rate: f32,
    window_sent: u64,
    window_recv: u64,
    window_pkts_sent: u32,
    window_pkts_recv: u32,
    window_t: f32,
    pub ping_ms: f32,
    /// Set when a packet from another build of the game arrived (its protocol byte).
    pub foreign_version: Option<u8>,
}

impl NetStats {
    pub fn tick(&mut self, dt: f32) {
        self.window_t += dt;
        if self.window_t >= 1.0 {
            let t = self.window_t;
            self.sent_rate = self.window_sent as f32 / t;
            self.recv_rate = self.window_recv as f32 / t;
            self.pkts_sent_rate = self.window_pkts_sent as f32 / t;
            self.pkts_recv_rate = self.window_pkts_recv as f32 / t;
            self.window_sent = 0;
            self.window_recv = 0;
            self.window_pkts_sent = 0;
            self.window_pkts_recv = 0;
            self.window_t = 0.0;
        }
    }
}

pub struct Sock {
    pub(crate) s: UdpSocket,
    pub stats: NetStats,
    buf: Vec<u8>,
    /// Fraction of outgoing packets to drop on purpose, for testing bad connections
    /// (ASHEN_NET_LOSS=0.1 drops 10%).
    loss: f32,
}

impl Sock {
    pub fn bind(port: u16) -> std::io::Result<Sock> {
        let s = UdpSocket::bind(("0.0.0.0", port))?;
        s.set_nonblocking(true)?;
        let loss = std::env::var("ASHEN_NET_LOSS").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0f32).clamp(0.0, 0.95);
        Ok(Sock { s, stats: NetStats::default(), buf: vec![0; 65536], loss })
    }

    #[cfg(test)]
    pub fn local_port(&self) -> u16 {
        self.s.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn send(&mut self, data: &[u8], to: SocketAddr) {
        if self.loss > 0.0 && macroquad::rand::gen_range(0.0, 1.0) < self.loss {
            return;
        }
        if self.s.send_to(data, to).is_ok() {
            // Count IP + UDP headers too, that's what actually goes over the wire.
            let n = data.len() as u64 + 28;
            self.stats.sent_total += n;
            self.stats.window_sent += n;
            self.stats.window_pkts_sent += 1;
        }
    }

    /// Next valid packet, or None when the socket is drained.
    pub fn recv(&mut self) -> Option<(PType, Vec<u8>, SocketAddr)> {
        loop {
            match self.s.recv_from(&mut self.buf) {
                Ok((n, from)) => {
                    let n64 = n as u64 + 28;
                    self.stats.recv_total += n64;
                    self.stats.window_recv += n64;
                    self.stats.window_pkts_recv += 1;
                    if n >= 4 && self.buf[..2] == MAGIC[..2] && self.buf[2] != MAGIC[2] {
                        self.stats.foreign_version = Some(self.buf[2]);
                        continue;
                    }
                    if n < 4 || self.buf[..3] != MAGIC {
                        continue;
                    }
                    let Some(t) = PType::from(self.buf[3]) else { continue };
                    return Some((t, self.buf[4..n].to_vec(), from));
                }
                // Windows reports ICMP "port unreachable" as ConnectionReset on UDP; ignore it.
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                Err(_) => return None,
            }
        }
    }
}

/// Splits a snapshot into fragments small enough to avoid IP fragmentation (Tailscale MTU is 1280).
pub fn fragments(tick: u32, payload: &[u8]) -> Vec<Vec<u8>> {
    let count = payload.len().div_ceil(FRAG_PAYLOAD).max(1);
    (0..count)
        .map(|i| {
            let mut w = Writer::packet(PType::Snap);
            w.u32(tick);
            w.u16(i as u16);
            w.u16(count as u16);
            let end = ((i + 1) * FRAG_PAYLOAD).min(payload.len());
            w.0.extend_from_slice(&payload[i * FRAG_PAYLOAD..end]);
            w.0
        })
        .collect()
}

/// Fragments of one snapshot: how many there are, and those that arrived.
type Partial = (u16, Vec<Option<Vec<u8>>>);

/// A player's log never travels in more than this many fragments, or unpacks bigger than this.
const MAX_LOG_FRAGMENTS: usize = 600;
const MAX_LOG_BYTES: usize = 2 << 20;

/// The socket and host a player's log goes to, set while joined to someone's vigil.
static UPLOAD: std::sync::Mutex<Option<(UdpSocket, SocketAddr)>> = std::sync::Mutex::new(None);

/// Stops sending logs anywhere, after leaving a host.
pub fn forget_upload() {
    if let Ok(mut up) = UPLOAD.lock() {
        *up = None;
    }
}

/// Sends this session's log to the host this player joined, if any: packed, in fragments,
/// twice over so a lost packet rarely loses the log. `reason` says why ("vigil", "crash").
pub fn upload_log(reason: &str) {
    let Ok(guard) = UPLOAD.try_lock() else { return };
    let Some((sock, host)) = guard.as_ref() else { return };
    let Some(text) = crate::log::current_text() else { return };
    let tail_from = text.len().saturating_sub(MAX_LOG_BYTES - 1024);
    let tail = &text[text.char_indices().map(|(i, _)| i).find(|&i| i >= tail_from).unwrap_or(0)..];
    let packed = miniz_oxide::deflate::compress_to_vec(format!("{}\n{}", reason, tail).as_bytes(), 6);
    let count = packed.len().div_ceil(FRAG_PAYLOAD).clamp(1, MAX_LOG_FRAGMENTS);
    let id = (crate::log::uptime_ms() as u32) ^ 0x5a5a;
    for _ in 0..2 {
        for i in 0..count {
            let mut w = Writer::packet(PType::Log);
            w.u32(id);
            w.u16(i as u16);
            w.u16(count as u16);
            let end = ((i + 1) * FRAG_PAYLOAD).min(packed.len());
            w.0.extend_from_slice(&packed[(i * FRAG_PAYLOAD).min(end)..end]);
            let _ = sock.send_to(&w.0, host);
        }
    }
    crate::log!("sent this log to the host ({}, {} bytes packed)", reason, packed.len());
}

/// Reassembles snapshot fragments; only the newest complete snapshot matters.
#[derive(Default)]
pub struct Assembler {
    parts: HashMap<u32, Partial>,
    pub last_tick: u32,
}

impl Assembler {
    pub fn push(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        let mut r = Reader::new(data);
        let tick = r.u32();
        let idx = r.u16();
        let count = r.u16();
        let body = r.rest();
        if !r.ok || count == 0 || idx >= count || (tick <= self.last_tick && self.last_tick != 0) {
            return None;
        }
        let entry = self.parts.entry(tick).or_insert_with(|| (count, vec![None; count as usize]));
        if entry.0 != count {
            return None;
        }
        entry.1[idx as usize] = Some(body.to_vec());
        if entry.1.iter().all(|p| p.is_some()) {
            let (_, parts) = self.parts.remove(&tick).unwrap();
            self.last_tick = tick;
            self.parts.retain(|t, _| *t > tick);
            return Some(parts.into_iter().flatten().flatten().collect());
        }
        // Drop stale half-finished snapshots.
        if self.parts.len() > 8 {
            let min = *self.parts.keys().min().unwrap();
            self.parts.remove(&min);
        }
        None
    }
}

/// Resolves "host", "host:port", an IPv4 address or a Tailscale MagicDNS name.
pub fn resolve(text: &str) -> Result<SocketAddr, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("Enter an address first".into());
    }
    let with_port = if t.parse::<SocketAddr>().is_ok() || t.rsplit_once(':').map(|(_, p)| p.parse::<u16>().is_ok()).unwrap_or(false) {
        t.to_string()
    } else {
        format!("{}:{}", t, game_port())
    };
    with_port.to_socket_addrs().map_err(|e| format!("Cannot resolve {}: {}", t, e))?.find(|a| a.is_ipv4()).ok_or_else(|| format!("No IPv4 address for {}", t))
}

/// Local address the OS would use to reach `target`. No packets are sent.
fn route_ip(target: &str) -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect(target).ok()?;
    s.local_addr().ok().map(|a| a.ip())
}

pub fn is_tailscale_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let o = v.octets();
            o[0] == 100 && (64..128).contains(&o[1])
        }
        _ => false,
    }
}

pub fn lan_ip() -> Option<IpAddr> {
    route_ip("8.8.8.8:80").filter(|ip| !is_tailscale_ip(*ip))
}

#[derive(Clone, Debug)]
pub struct TsPeer {
    pub name: String,
    pub ip: String,
    pub online: bool,
    pub os: String,
}

#[derive(Clone, Debug, Default)]
pub struct TsStatus {
    pub installed: bool,
    pub running: bool,
    pub self_ip: Option<String>,
    pub self_name: String,
    pub peers: Vec<TsPeer>,
    pub error: Option<String>,
}

fn tailscale_cmd() -> Option<std::process::Command> {
    let candidates = ["tailscale", r"C:\Program Files\Tailscale\tailscale.exe", "/usr/bin/tailscale", "/Applications/Tailscale.app/Contents/MacOS/Tailscale"];
    for c in candidates {
        let mut cmd = std::process::Command::new(c);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.arg("version");
        if cmd.output().map(|o| o.status.success()).unwrap_or(false) {
            let mut real = std::process::Command::new(c);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                real.creation_flags(0x0800_0000);
            }
            return Some(real);
        }
    }
    None
}

/// Asks the local Tailscale client for its state and peers. Blocking; call from a thread.
pub fn tailscale_status() -> TsStatus {
    let mut st = TsStatus::default();
    // Even without the CLI, a route through 100.100.100.100 reveals an active tailnet.
    let routed = route_ip("100.100.100.100:53").filter(|ip| is_tailscale_ip(*ip));
    let Some(mut cmd) = tailscale_cmd() else {
        st.installed = routed.is_some();
        st.running = routed.is_some();
        st.self_ip = routed.map(|i| i.to_string());
        if routed.is_none() {
            st.error = Some("Tailscale not found. Install it from tailscale.com".into());
        }
        return st;
    };
    st.installed = true;
    let out = match cmd.args(["status", "--json"]).output() {
        Ok(o) => o,
        Err(e) => {
            st.error = Some(e.to_string());
            return st;
        }
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        st.error = Some("Tailscale is not running".into());
        return st;
    };
    st.running = v["BackendState"].as_str() == Some("Running");
    if !st.running {
        // Tailscale's own state names mean little to players.
        st.error = Some(match v["BackendState"].as_str().unwrap_or("") {
            "NoState" | "Stopped" => "not connected, open Tailscale and connect".to_string(),
            "NeedsLogin" | "NeedsMachineAuth" => "signed out, sign in to Tailscale".to_string(),
            "Starting" => "starting...".to_string(),
            other => format!("state {}", if other.is_empty() { "unknown" } else { other }),
        });
    }
    let first_v4 = |n: &serde_json::Value| -> Option<String> { n["TailscaleIPs"].as_array()?.iter().filter_map(|x| x.as_str()).find(|s| s.contains('.')).map(|s| s.to_string()) };
    st.self_ip = first_v4(&v["Self"]).or(routed.map(|i| i.to_string()));
    st.self_name = v["Self"]["HostName"].as_str().unwrap_or("").to_string();
    if let Some(peers) = v["Peer"].as_object() {
        for p in peers.values() {
            let Some(ip) = first_v4(p) else { continue };
            st.peers.push(TsPeer {
                name: p["HostName"].as_str().unwrap_or("?").to_string(),
                ip,
                online: p["Online"].as_bool().unwrap_or(false),
                os: p["OS"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    st.peers.sort_by(|a, b| b.online.cmp(&a.online).then(a.name.cmp(&b.name)));
    st
}

fn write_profile(w: &mut Writer, p: &crate::data::Profile) {
    for m in p.meta {
        w.u8(m as u8);
    }
    for r in p.relics {
        w.u8(r as u8);
    }
    w.u32(p.unlocks as u32);
    w.u32((p.unlocks >> 32) as u32);
    w.u8(p.skin);
}

/// Reads a joining player's progression, clamped to what the game allows.
fn read_profile(r: &mut Reader) -> crate::data::Profile {
    use crate::data::*;
    let mut p = Profile::default();
    for (i, m) in p.meta.iter_mut().enumerate() {
        *m = (r.u8() as u32).min(META[i].max);
    }
    for (i, x) in p.relics.iter_mut().enumerate() {
        *x = (r.u8() as u32).min(RELICS[i].max);
    }
    let lo = r.u32() as u64;
    let hi = r.u32() as u64;
    p.unlocks = lo | hi << 32 | default_unlocks();
    p.skin = r.u8().min(SKINS.len() as u8);
    p
}

/// Player in the lobby as the host announces them.
#[derive(Clone, Debug)]
pub struct LobbyPlayer {
    pub slot: u8,
    pub name: String,
    pub class: u8,
}

/// Input a client sends every frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClientInput {
    pub pos: (f32, f32),
    pub facing: (f32, f32),
    pub moving: bool,
    pub roll_t: f32,
    pub roll_dir: (f32, f32),
    pub stamina: f32,
    pub pick_round: u16,
    pub pick_idx: u8,
    pub reroll_seq: u8,
    pub rolls: u32,
    /// Counts skill presses; the host casts when it changes.
    pub skill_seq: u8,
    /// Newest snapshot applied, so the host can code the next one against it.
    pub ack: u32,
    /// Window size, so the host only sends what fits on screen.
    pub view: (u16, u16),
    /// The player has the menu open; when everyone has, the host holds the world.
    pub menu: bool,
}

impl ClientInput {
    pub fn write(&self, w: &mut Writer) {
        w.f32(self.pos.0);
        w.f32(self.pos.1);
        w.f32(self.facing.0);
        w.f32(self.facing.1);
        w.bool(self.moving);
        w.f32(self.roll_t);
        w.f32(self.roll_dir.0);
        w.f32(self.roll_dir.1);
        w.f32(self.stamina);
        w.u16(self.pick_round);
        w.u8(self.pick_idx);
        w.u8(self.reroll_seq);
        w.u32(self.rolls);
        w.u8(self.skill_seq);
        w.u32(self.ack);
        w.u16(self.view.0);
        w.u16(self.view.1);
        w.bool(self.menu);
    }

    pub fn read(r: &mut Reader) -> ClientInput {
        ClientInput {
            pos: (r.f32(), r.f32()),
            facing: (r.f32(), r.f32()),
            moving: r.bool(),
            roll_t: r.f32(),
            roll_dir: (r.f32(), r.f32()),
            stamina: r.f32(),
            pick_round: r.u16(),
            pick_idx: r.u8(),
            reroll_seq: r.u8(),
            rolls: r.u32(),
            skill_seq: r.u8(),
            ack: r.u32(),
            view: (r.u16(), r.u16()),
            menu: r.bool(),
        }
    }
}

pub struct Peer {
    pub addr: SocketAddr,
    pub slot: u8,
    pub name: String,
    pub class: u8,
    pub profile: crate::data::Profile,
    pub last_seen: f64,
    pub ping_ms: f32,
    pub input: Option<ClientInput>,
    pub snap: crate::snapshot::Encoder,
}

pub struct HostNet {
    pub sock: Sock,
    pub peers: Vec<Peer>,
    pub started: bool,
    pub tick: u32,
    snap_acc: f32,
    ping_acc: f32,
    lobby_acc: f32,
    /// Players who dropped out of the running vigil, by name and slot; the same name may
    /// come back and take its hero over again.
    pub left: Vec<(String, u8)>,
    /// Players' logs arriving in fragments, by sender and log id.
    logs_in: HashMap<(SocketAddr, u32), Partial>,
    /// Logs already stored; their second copy is ignored.
    logs_done: Vec<(SocketAddr, u32)>,
}

pub enum HostEvent {
    Joined(String),
    /// Name and slot; names are not unique.
    Left(String, u8),
    /// A player who dropped out is back in the running vigil.
    Rejoined(String, u8),
    /// A player's log arrived: name, why it was sent, the text.
    Log(String, String, String),
}

impl HostNet {
    pub fn open(port: u16) -> Result<HostNet, String> {
        let sock = Sock::bind(port).map_err(|e| format!("Cannot open port {}: {}", port, e))?;
        Ok(HostNet {
            sock,
            peers: Vec::new(),
            started: false,
            tick: 0,
            snap_acc: 0.0,
            ping_acc: 0.0,
            lobby_acc: 0.0,
            left: Vec::new(),
            logs_in: HashMap::new(),
            logs_done: Vec::new(),
        })
    }

    /// Handles incoming packets. `lobby` is the current player list for new joiners.
    pub fn poll(&mut self, now: f64) -> Vec<HostEvent> {
        let mut events = Vec::new();
        while let Some((t, data, from)) = self.sock.recv() {
            let mut r = Reader::new(&data);
            let known = self.peers.iter().position(|p| p.addr == from);
            match t {
                PType::Hello => {
                    let name: String = r.str().chars().take(16).collect();
                    let class = r.u8();
                    let profile = read_profile(&mut r);
                    let slot = match known {
                        Some(i) => self.peers[i].slot,
                        None if self.started && self.left.iter().any(|(n, _)| *n == name) => {
                            let k = self.left.iter().position(|(n, _)| *n == name).unwrap_or(0);
                            let slot = self.left.remove(k).1;
                            events.push(HostEvent::Rejoined(name.clone(), slot));
                            self.peers.push(Peer { addr: from, slot, name: name.clone(), class, profile, last_seen: now, ping_ms: 0.0, input: None, snap: Default::default() });
                            slot
                        }
                        None => {
                            if self.started || self.peers.len() >= crate::world::MAX_HEROES - 1 {
                                let mut w = Writer::packet(PType::Refused);
                                w.str(if self.started { "The vigil has already begun" } else { "The lobby is full" });
                                self.sock.send(&w.0, from);
                                continue;
                            }
                            let slot = (1..4u8).find(|s| !self.peers.iter().any(|p| p.slot == *s)).unwrap_or(1);
                            events.push(HostEvent::Joined(name.clone()));
                            self.peers.push(Peer { addr: from, slot, name: name.clone(), class, profile, last_seen: now, ping_ms: 0.0, input: None, snap: Default::default() });
                            slot
                        }
                    };
                    if let Some(i) = self.peers.iter().position(|p| p.addr == from) {
                        let p = &mut self.peers[i];
                        p.name = name;
                        p.class = class;
                        p.profile = profile;
                        p.last_seen = now;
                    }
                    let mut w = Writer::packet(PType::Welcome);
                    w.u8(slot);
                    self.sock.send(&w.0, from);
                }
                PType::Input => {
                    if let Some(i) = known {
                        let inp = ClientInput::read(&mut r);
                        if r.ok {
                            self.peers[i].input = Some(inp);
                        }
                        self.peers[i].last_seen = now;
                    }
                }
                PType::Ping => {
                    let t = r.f64();
                    let mut w = Writer::packet(PType::Pong);
                    w.f64(t);
                    self.sock.send(&w.0, from);
                    if let Some(i) = known {
                        self.peers[i].last_seen = now;
                    }
                }
                PType::Pong => {
                    if let Some(i) = known {
                        self.peers[i].ping_ms = ((now - r.f64()) * 1000.0) as f32;
                        self.peers[i].last_seen = now;
                    }
                }
                PType::Log => {
                    // Only from players in this lobby, and never more than a few at once.
                    if let (Some(i), true) = (known, self.logs_in.len() < 8) {
                        let (id, idx, count) = (r.u32(), r.u16() as usize, r.u16() as usize);
                        let body = r.rest().to_vec();
                        if !r.ok || count == 0 || count > MAX_LOG_FRAGMENTS || idx >= count || self.logs_done.contains(&(from, id)) {
                            continue;
                        }
                        let entry = self.logs_in.entry((from, id)).or_insert_with(|| (count as u16, vec![None; count]));
                        if entry.1.len() != count {
                            continue;
                        }
                        entry.1[idx] = Some(body);
                        if entry.1.iter().all(|p| p.is_some()) {
                            let (_, parts) = self.logs_in.remove(&(from, id)).unwrap_or_default();
                            self.logs_done.push((from, id));
                            if self.logs_done.len() > 32 {
                                self.logs_done.remove(0);
                            }
                            let packed: Vec<u8> = parts.into_iter().flatten().flatten().collect();
                            if let Ok(raw) = miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, MAX_LOG_BYTES) {
                                let text = String::from_utf8_lossy(&raw);
                                let (reason, text) = text.split_once('\n').unwrap_or(("log", &text));
                                events.push(HostEvent::Log(self.peers[i].name.clone(), reason.to_string(), text.to_string()));
                            }
                        }
                    }
                }
                PType::Leave => {
                    if let Some(i) = known {
                        events.push(HostEvent::Left(self.peers[i].name.clone(), self.peers[i].slot));
                        let p = self.peers.remove(i);
                        if self.started {
                            self.left.push((p.name, p.slot));
                        }
                    }
                }
                _ => {}
            }
        }
        for p in self.peers.iter().filter(|p| now - p.last_seen >= TIMEOUT) {
            events.push(HostEvent::Left(p.name.clone(), p.slot));
            if self.started {
                self.left.push((p.name.clone(), p.slot));
            }
        }
        self.peers.retain(|p| now - p.last_seen < TIMEOUT);
        let n = self.peers.len().max(1) as f32;
        self.sock.stats.ping_ms = self.peers.iter().map(|p| p.ping_ms).sum::<f32>() / n;
        events
    }

    pub fn lobby_players(&self, host_name: &str, host_class: u8) -> Vec<LobbyPlayer> {
        let mut v = vec![LobbyPlayer { slot: 0, name: host_name.to_string(), class: host_class }];
        for p in &self.peers {
            v.push(LobbyPlayer { slot: p.slot, name: p.name.clone(), class: p.class });
        }
        v.sort_by_key(|p| p.slot);
        v
    }

    /// Periodic lobby broadcast and pings.
    #[allow(clippy::too_many_arguments)]
    pub fn tick_lobby(&mut self, dt: f32, now: f64, players: &[LobbyPlayer], diff: &crate::data::Difficulty, rush: bool, challenges: u8, biome: u8, curses: u8) {
        self.lobby_acc += dt;
        if self.lobby_acc >= 0.25 {
            self.lobby_acc = 0.0;
            let mut w = Writer::packet(PType::Lobby);
            w.bool(self.started);
            w.u8(players.len() as u8);
            for p in players {
                w.u8(p.slot);
                w.str(&p.name);
                w.u8(p.class);
            }
            for v in [diff.count, diff.hp, diff.dmg, diff.speed, diff.boss_hp] {
                w.f32(v);
            }
            for b in diff.boost {
                w.u8(b);
            }
            w.bool(rush);
            w.u8(challenges);
            w.u8(biome);
            w.u8(curses);
            let addrs: Vec<SocketAddr> = self.peers.iter().map(|p| p.addr).collect();
            for a in addrs {
                self.sock.send(&w.0, a);
            }
        }
        self.ping_acc += dt;
        if self.ping_acc >= 0.5 {
            self.ping_acc = 0.0;
            let mut w = Writer::packet(PType::Ping);
            w.f64(now);
            let addrs: Vec<SocketAddr> = self.peers.iter().map(|p| p.addr).collect();
            for a in addrs {
                self.sock.send(&w.0, a);
            }
        }
        self.sock.stats.tick(dt);
    }

    /// True when it is time to send the next snapshot.
    pub fn snapshot_due(&mut self, dt: f32, paused: bool) -> bool {
        let interval = 1.0 / if paused { PAUSED_SNAPSHOT_RATE } else { SNAPSHOT_RATE };
        self.snap_acc += dt;
        if self.snap_acc >= interval {
            // Keep the remainder so the rate holds at any frame rate; never bank more than one.
            self.snap_acc = (self.snap_acc - interval).min(interval);
            true
        } else {
            false
        }
    }

    /// Encodes and sends this round's snapshot to every client, each cut to its own view.
    pub fn send_snapshots(&mut self, w: &mut crate::world::World) {
        use crate::snapshot::{begin_round, View};
        self.tick += 1;
        let ev = begin_round(w);
        for p in self.peers.iter_mut() {
            let (ack, screen) = p.input.map(|i| (i.ack, macroquad::prelude::vec2(i.view.0 as f32, i.view.1 as f32))).unwrap_or_default();
            let data = p.snap.encode(w, &ev, &View::of(w, p.slot, screen), self.tick, ack);
            for f in fragments(self.tick, &data) {
                self.sock.send(&f, p.addr);
            }
        }
    }

    pub fn close(&mut self) {
        let w = Writer::packet(PType::Leave);
        let addrs: Vec<SocketAddr> = self.peers.iter().map(|p| p.addr).collect();
        for a in addrs {
            self.sock.send(&w.0, a);
        }
    }
}

pub enum ClientEvent {
    Welcome,
    Refused(String),
    Lobby { started: bool, players: Vec<LobbyPlayer>, diff: crate::data::Difficulty, rush: bool, challenges: u8, biome: u8, curses: u8 },
    Snapshot(Vec<u8>),
    HostLeft,
}

pub struct ClientNet {
    pub sock: Sock,
    pub host: SocketAddr,
    pub slot: Option<u8>,
    pub last_seen: f64,
    assembler: Assembler,
    hello_acc: f32,
    ping_acc: f32,
    /// HostLeft was reported already; it is said once, not every frame after.
    gone: bool,
}

impl ClientNet {
    pub fn connect(host: SocketAddr, now: f64) -> Result<ClientNet, String> {
        let sock = Sock::bind(0).map_err(|e| format!("Cannot open a socket: {}", e))?;
        // Kept for sending the log to this host, even from inside a crash.
        if let (Ok(s), Ok(mut up)) = (sock.s.try_clone(), UPLOAD.lock()) {
            *up = Some((s, host));
        }
        Ok(ClientNet { sock, host, slot: None, last_seen: now, assembler: Assembler::default(), hello_acc: 10.0, ping_acc: 0.0, gone: false })
    }

    /// Re-sends the join request until the host answers.
    pub fn tick(&mut self, dt: f32, now: f64, name: &str, class: u8, profile: &crate::data::Profile) {
        self.hello_acc += dt;
        if self.slot.is_none() && self.hello_acc >= 0.4 {
            self.hello_acc = 0.0;
            let mut w = Writer::packet(PType::Hello);
            w.str(name);
            w.u8(class);
            write_profile(&mut w, profile);
            self.sock.send(&w.0, self.host);
        }
        self.ping_acc += dt;
        if self.ping_acc >= 0.5 {
            self.ping_acc = 0.0;
            let mut w = Writer::packet(PType::Ping);
            w.f64(now);
            self.sock.send(&w.0, self.host);
        }
        self.sock.stats.tick(dt);
    }

    pub fn poll(&mut self, now: f64) -> Vec<ClientEvent> {
        let mut out = Vec::new();
        let mut latest_snap = None;
        while let Some((t, data, from)) = self.sock.recv() {
            if from != self.host {
                continue;
            }
            self.last_seen = now;
            let mut r = Reader::new(&data);
            match t {
                PType::Welcome => {
                    let s = r.u8();
                    if self.slot.is_none() {
                        out.push(ClientEvent::Welcome);
                    }
                    self.slot = Some(s);
                }
                PType::Refused => out.push(ClientEvent::Refused(r.str())),
                PType::Lobby => {
                    let started = r.bool();
                    let n = r.u8();
                    let mut players = Vec::new();
                    for _ in 0..n {
                        players.push(LobbyPlayer { slot: r.u8(), name: r.str(), class: r.u8() });
                    }
                    let mut diff = crate::data::Difficulty { count: r.f32(), hp: r.f32(), dmg: r.f32(), speed: r.f32(), boss_hp: r.f32(), boost: [0; 3] };
                    for i in 0..3 {
                        diff.boost[i] = r.u8().min(crate::data::BOOST_MAX[i]);
                    }
                    let rush = r.bool();
                    let challenges = r.u8() & 7;
                    let biome = r.u8().min(ALL_FIELDS_MAX);
                    let curses = r.u8() & 63;
                    if r.ok {
                        out.push(ClientEvent::Lobby { started, players, diff, rush, challenges, biome, curses });
                    }
                }
                PType::Snap => {
                    if let Some(s) = self.assembler.push(&data) {
                        latest_snap = Some(s);
                    }
                }
                PType::Ping => {
                    let t = r.f64();
                    let mut w = Writer::packet(PType::Pong);
                    w.f64(t);
                    self.sock.send(&w.0, self.host);
                }
                PType::Pong => self.sock.stats.ping_ms = ((now - r.f64()) * 1000.0) as f32,
                PType::Leave => out.push(ClientEvent::HostLeft),
                _ => {}
            }
        }
        if let Some(s) = latest_snap {
            out.push(ClientEvent::Snapshot(s));
        }
        if now - self.last_seen > TIMEOUT {
            out.push(ClientEvent::HostLeft);
        }
        if out.iter().any(|e| matches!(e, ClientEvent::HostLeft)) {
            out.retain(|e| !matches!(e, ClientEvent::HostLeft));
            if !self.gone {
                self.gone = true;
                out.push(ClientEvent::HostLeft);
            }
        }
        out
    }

    pub fn send_input(&mut self, inp: &ClientInput) {
        let mut w = Writer::packet(PType::Input);
        inp.write(&mut w);
        self.sock.send(&w.0, self.host);
    }

    pub fn leave(&mut self) {
        let w = Writer::packet(PType::Leave);
        self.sock.send(&w.0, self.host);
    }
}
