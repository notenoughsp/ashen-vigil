// Release builds are a window program: no console window opens next to the game.
// Everything worth reading goes to the log file anyway.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod data;
mod discord;
mod draw;
mod embedded;
mod icon_art;
mod icons;
mod log;
mod net;
mod save;
mod snapshot;
#[cfg(test)]
mod tests;
mod ui;
mod update;
mod voice;
mod world;

use app::{App, AutoTest};
use audio::Audio;
use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;
use save::{Save, Settings};
use std::time::{Duration, Instant};

/// Window icon at one size, from the same drawing as the executable's icon.
fn icon_pixels<const N: usize>(size: usize) -> [u8; N] {
    let mut px = [0u8; N];
    px.copy_from_slice(&icon_art::icon_rgba(size));
    px
}

/// Window settings come from the save file, so resolution, VSync and MSAA apply at start.
fn conf() -> Conf {
    // Tests use default window settings and measure without VSync, whatever the player's save says.
    let mut st = if std::env::var("ASHEN_AUTOTEST").is_ok() { Settings { vsync: false, ..Settings::default() } } else { Save::load().settings };
    // "WxH" from ASHEN_WINDOW_SIZE, so the tests can check large screens.
    if let Some((w, h)) = std::env::var("ASHEN_WINDOW_SIZE").ok().and_then(|v| v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))) {
        st.resolution = (w, h);
    }
    let mut c = Conf {
        window_title: "Ashen Vigil".to_string(),
        window_width: st.resolution.0 as i32,
        window_height: st.resolution.1 as i32,
        window_resizable: true,
        fullscreen: st.fullscreen,
        sample_count: st.msaa as i32,
        icon: Some(Icon { small: icon_pixels::<1024>(16), medium: icon_pixels::<4096>(32), big: icon_pixels::<16384>(64) }),
        ..Default::default()
    };
    c.platform.swap_interval = Some(if st.vsync { 1 } else { 0 });
    c
}

/// "x,y" from ASHEN_WINDOW_POS, used by the tests to open on another monitor.
fn window_pos() -> Option<(i32, i32)> {
    let v = std::env::var("ASHEN_WINDOW_POS").ok()?;
    let (x, y) = v.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(period: u32) -> u32;
}

/// Lets `thread::sleep` wake up within a millisecond instead of Windows' default 15.6 ms.
fn precise_sleep_timer() {
    #[cfg(windows)]
    // SAFETY: plain Win32 call without pointers; it only changes the system timer resolution.
    unsafe {
        timeBeginPeriod(1);
    }
}

/// Waits out the rest of the frame for the FPS cap: sleep most of it, spin the last bit.
fn limit_fps(frame_start: Instant, cap: u32) {
    if cap == 0 {
        return;
    }
    let target = Duration::from_secs_f64(1.0 / cap as f64);
    loop {
        let elapsed = frame_start.elapsed();
        if elapsed >= target {
            break;
        }
        let left = target - elapsed;
        if left > Duration::from_millis(2) {
            std::thread::sleep(left - Duration::from_millis(1));
        } else {
            std::hint::spin_loop();
        }
    }
}

#[macroquad::main(conf)]
async fn main() {
    // Assets are looked up relative to the working directory; when the game is started
    // from a shortcut or another folder, use the folder the exe lives in instead.
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        if dir.join("assets").is_dir() {
            let _ = std::env::set_current_dir(dir);
        }
    }
    precise_sleep_timer();
    rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    let auto = AutoTest::from_env();
    // The self-test logs next to its screenshots, so it never pushes the player's logs out.
    log::init(&log::log_dir(), auto.as_ref().map(|a| a.log_file()));
    let mut save = Save::load();
    let mut audio = Audio::new().await;
    if let Some(au) = &auto {
        // Tests never touch the real save and never make a sound.
        save = Save { readonly: true, ..Save::default() };
        save.settings.name = au.player_name().into();
        save.profile.unlocks = u64::MAX;
        audio.toggle_mute();
    }
    audio.sfx_vol = save.settings.sfx;
    audio.kind_vol = save.settings.vols;
    audio.music_vol = save.settings.music;
    let font_bytes = std::fs::read("assets/font.ttf").unwrap_or_else(|_| embedded::FONT.to_vec());
    let font = load_ttf_font_from_bytes(&font_bytes).ok();
    {
        let st = &save.settings;
        crate::log!(
            "window {}x{} (asked {}x{}), fullscreen {}, vsync {}, fps cap {}, ui scale setting {}, lighting {}, msaa {}",
            screen_width(),
            screen_height(),
            st.resolution.0,
            st.resolution.1,
            st.fullscreen,
            st.vsync,
            st.fps_cap,
            st.ui_scale,
            st.lighting,
            st.msaa
        );
        crate::log!("save: {} vigils, {} souls, {} marks", save.vigils, save.souls, save.marks);
    }
    let mut app = App::new(save, audio, font, auto);
    // Frame times, summed up in the log once a minute.
    let (mut perf_t, mut perf_frames, mut perf_worst) = (0.0f32, 0u32, 0.0f32);
    let mut placed = window_pos().is_none();
    let mut frame_start = Instant::now();
    loop {
        if !placed {
            // Negative coordinates (monitors left of the primary) survive the u32 round trip.
            let (x, y) = window_pos().unwrap();
            macroquad::miniquad::window::set_window_position(x as u32, y as u32);
            placed = true;
        }
        let raw_dt = get_frame_time();
        let dt = raw_dt.min(1.0 / 30.0);
        perf_t += raw_dt;
        perf_frames += 1;
        perf_worst = perf_worst.max(raw_dt);
        if perf_t >= 60.0 {
            crate::log!("perf: {:.0} fps average, slowest frame {:.0} ms; {}", perf_frames as f32 / perf_t, perf_worst * 1000.0, app.log_status());
            (perf_t, perf_frames, perf_worst) = (0.0, 0, 0.0);
        }
        app.update(dt);
        app.draw(dt);
        app.autotest();
        limit_fps(frame_start, app.save.settings.fps_cap);
        frame_start = Instant::now();
        next_frame().await;
    }
}
