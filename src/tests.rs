//! Headless stress and robustness tests. Run with `cargo test --release -- --nocapture`.

use crate::data::*;
use crate::net::{self, Assembler, Reader};
use crate::save::Save;
use crate::snapshot;
use crate::world::*;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::collections::HashMap;
use std::time::Instant;

/// Largest numbers seen during a simulation, to catch runaway growth.
#[derive(Default, Debug)]
struct Peaks {
    enemies: usize,
    particles: usize,
    decals: usize,
    pickups: usize,
    bullets: usize,
    projs: usize,
    worst_update_ms: f64,
    avg_update_ms: f64,
}

fn world_with(heroes: usize, diff: Difficulty) -> World {
    let mut w = World::new(diff, None);
    w.screen = vec2(1920.0, 1080.0);
    let profile = Profile { unlocks: u64::MAX, ..Profile::default() };
    for i in 0..heroes {
        let class = ALL_CLASSES[i % ALL_CLASSES.len()];
        w.add_hero(i as u8, &format!("H{}", i), class, profile, false);
    }
    w
}

/// Simulates `secs` of play. Heroes walk in circles, roll, and pick the first card.
fn simulate(w: &mut World, secs: f32, immortal: bool, mut on_frame: impl FnMut(&mut World, u32)) -> Peaks {
    let dt = 1.0 / 30.0;
    let frames = (secs / dt) as u32;
    let mut peaks = Peaks::default();
    let mut total_ms = 0.0;
    let mut updates = 0u32;
    for f in 0..frames {
        if w.pause == Pause::Choosing {
            for h in 0..w.heroes.len() {
                let round = w.level_round;
                w.pick(h, round, 0);
            }
        }
        for (i, h) in w.heroes.iter_mut().enumerate() {
            let t = f as f32 * dt + i as f32;
            h.input = Input { dir: vec2(t.cos(), (t * 0.7).sin()).normalize_or_zero(), roll: f % (40 + i as u32) == 0, skill: f % (300 + i as u32) == 0 };
            if immortal {
                // Full health alone isn't enough: one Abyss boss hit can exceed it.
                h.p.hp = h.stats.max_hp;
                h.p.iframes = 1.0;
            }
        }
        let start = Instant::now();
        w.update(dt);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        total_ms += ms;
        updates += 1;
        peaks.worst_update_ms = peaks.worst_update_ms.max(ms);
        peaks.enemies = peaks.enemies.max(w.enemies.len());
        peaks.particles = peaks.particles.max(w.particles.len());
        peaks.decals = peaks.decals.max(w.decals.len());
        peaks.pickups = peaks.pickups.max(w.pickups.len());
        peaks.bullets = peaks.bullets.max(w.ebullets.len());
        peaks.projs = peaks.projs.max(w.projs.len());
        w.sounds.clear();
        on_frame(w, f);
        if w.dead || w.won {
            break;
        }
    }
    peaks.avg_update_ms = total_ms / updates.max(1) as f64;
    peaks
}

fn assert_bounded(p: &Peaks) {
    assert!(p.enemies <= 1700, "enemies ran away: {:?}", p);
    assert!(p.particles <= 3000, "particles ran away: {:?}", p);
    assert!(p.decals <= 900, "decals ran away: {:?}", p);
    assert!(p.pickups <= 700, "pickups ran away: {:?}", p);
}

#[test]
fn full_vigil_on_abyss_stays_bounded() {
    let mut w = world_with(1, PRESETS[3].1);
    let peaks = simulate(&mut w, VICTORY_TIME + 400.0, true, |_, _| {});
    println!("single abyss: t={:.0}s level={} kills={} bosses={} won={} {:?}", w.time, w.level, w.kills, w.bosses_killed, w.won, peaks);
    assert_bounded(&peaks);
    assert!(w.time >= VICTORY_TIME - 1.0 || w.won, "the vigil stopped early at {}", w.time);
    for h in &w.heroes {
        assert!(h.p.pos.x.abs() <= WORLD_HALF && h.p.pos.y.abs() <= WORLD_HALF, "hero left the field: {:?}", h.p.pos);
    }
    for e in &w.enemies {
        assert!(e.pos.is_finite(), "enemy position became NaN");
    }
}

#[test]
fn four_heroes_with_snapshots_every_frame() {
    let mut host = world_with(4, PRESETS[2].1);
    host.record_net = true;
    let mut client = World::new(host.diff, None);
    client.is_client = true;
    let mut enc = snapshot::Encoder::default();
    let mut dec = snapshot::Decoder::default();
    let mut max_bytes = 0;
    let mut tick = 0;
    let peaks = simulate(&mut host, 900.0, true, |w, f| {
        if f % 2 == 0 {
            tick += 1;
            let ev = snapshot::begin_round(w);
            let view = snapshot::View::of(w, 2, vec2(1920.0, 1080.0));
            let data = enc.encode(w, &ev, &view, tick, dec.ack);
            max_bytes = max_bytes.max(data.len());
            assert!(dec.decode(&mut client, &data, 2, 1.0 / 15.0), "decode failed at tick {}", tick);
            client.client_update(1.0 / 30.0);
        }
    });
    println!("coop x4: t={:.0}s level={} kills={} max snapshot {} bytes ({} fragments) {:?}", host.time, host.level, host.kills, max_bytes, max_bytes.div_ceil(1100), peaks);
    assert_bounded(&peaks);
    assert_eq!(client.heroes.len(), 4);
    assert_eq!(client.me().slot, 2);
    assert_eq!(client.level, host.level);
}

#[test]
fn mortal_hero_dies_and_the_vigil_ends() {
    let mut w = world_with(1, PRESETS[3].1);
    for h in w.heroes.iter_mut() {
        h.p.revives = 0;
    }
    simulate(&mut w, 1200.0, false, |_, _| {});
    assert!(w.dead, "nobody died in 20 minutes of Abyss without healing");
    assert!(w.events.contains(&Event::Died));
}

#[test]
fn last_player_leaving_while_others_are_down_ends_the_vigil() {
    let mut w = world_with(2, Difficulty::default());
    w.heroes[1].remote = true;
    w.heroes[0].downed = true;
    w.disconnect(1);
    assert!(w.dead, "the vigil kept running with nobody left standing");
}

#[test]
fn level_up_round_does_not_wait_for_players_who_left() {
    let mut w = world_with(3, Difficulty::default());
    w.disconnect(2);
    w.pending_levels = 1;
    w.update(0.01);
    assert_eq!(w.pause, Pause::Choosing);
    let round = w.level_round;
    w.pick(0, round, 0);
    w.pick(1, round, 0);
    w.update(0.01);
    assert!(matches!(w.pause, Pause::Countdown(_) | Pause::Running), "still waiting on a player who left");
}

#[test]
fn fusion_merges_partners_and_frees_a_slot() {
    let mut w = world_with(1, Difficulty::default());
    let h = &mut w.heroes[0];
    h.weapons = vec![Weapon::new(WeaponKind::Blade), Weapon::new(WeaponKind::Orb), Weapon::new(WeaponKind::Axe)];
    for wp in h.weapons.iter_mut() {
        wp.level = WEAPON_MAX_LEVEL;
    }
    w.check_fusion(0);
    let h = &w.heroes[0];
    assert_eq!(h.weapons.len(), 2);
    assert!(h.weapons.iter().any(|wp| wp.kind == WeaponKind::Blade && wp.fused == vec![WeaponKind::Axe]));
}

#[test]
fn hostile_choice_indices_are_harmless() {
    let mut w = world_with(1, Difficulty::default());
    let h = &mut w.heroes[0];
    for c in [Choice::UpWeapon(250), Choice::UpItem(99), Choice::decode(9, 255), Choice::decode(0, 255), Choice::decode(2, 200)] {
        let _ = h.choice_text(c);
        h.apply_choice(c);
    }
    w.pick(0, 12345, 99);
    w.reroll(0);
    w.pick(7, 0, 0);
}

#[test]
fn snapshot_decoder_survives_garbage() {
    let mut host = world_with(2, Difficulty::default());
    host.record_net = true;
    simulate(&mut host, 120.0, true, |_, _| {});
    let mut enc = snapshot::Encoder::default();
    let ev = snapshot::begin_round(&mut host);
    let view = snapshot::View::of(&host, 1, vec2(4000.0, 4000.0));
    let real = enc.encode(&host, &ev, &view, 1, 0);
    // A delta snapshot against tick 1, for a decoder that has tick 1.
    simulate(&mut host, 0.2, true, |_, _| {});
    let delta = enc.encode(&host, &ev, &view, 2, 1);
    let fresh = |data: &[u8]| {
        let mut c = World::new(Difficulty::default(), None);
        c.is_client = true;
        let mut dec = snapshot::Decoder::default();
        assert!(dec.decode(&mut c, data, 1, 0.03));
        (c, dec)
    };
    // Every truncation of a real snapshot, full and delta.
    for len in 0..real.len() {
        let mut c = World::new(Difficulty::default(), None);
        c.is_client = true;
        let _ = snapshot::Decoder::default().decode(&mut c, &real[..len], 1, 0.03);
    }
    for len in 0..delta.len() {
        let (mut c, mut dec) = fresh(&real);
        let _ = dec.decode(&mut c, &delta[..len], 1, 0.03);
        c.client_update(0.03);
    }
    // Inflated bodies with random bytes flipped, so the garbage reaches the decoder itself.
    for _ in 0..1500 {
        let mut raw = miniz_oxide::inflate::decompress_to_vec(&delta).unwrap();
        for _ in 0..gen_range(1, 12) {
            let k = gen_range(8, raw.len());
            raw[k] = gen_range(0u8, 255u8);
        }
        let (mut c, mut dec) = fresh(&real);
        let _ = dec.decode(&mut c, &miniz_oxide::deflate::compress_to_vec(&raw, 1), 1, 0.03);
        c.client_update(0.03);
    }
    // Real snapshots with random bytes flipped, and pure noise.
    for i in 0..3000 {
        let mut data = if i % 2 == 0 { real.clone() } else { (0..gen_range(0, 3000)).map(|_| gen_range(0u8, 255u8)).collect() };
        for _ in 0..gen_range(1, 20) {
            if !data.is_empty() {
                let k = gen_range(0, data.len());
                data[k] = gen_range(0u8, 255u8);
            }
        }
        let mut c = World::new(Difficulty::default(), None);
        c.is_client = true;
        let _ = snapshot::Decoder::default().decode(&mut c, &data, 1, 0.03);
        c.client_update(0.03);
    }
}

#[test]
fn fragment_assembler_survives_garbage_and_loss() {
    let payload: Vec<u8> = (0..20_000).map(|i| (i % 251) as u8).collect();
    let frags = net::fragments(7, &payload);
    // In order, with the header stripped the way Sock::recv does.
    let mut a = Assembler::default();
    let mut out = None;
    for f in &frags {
        out = out.or(a.push(&f[4..]));
    }
    assert_eq!(out.as_deref(), Some(&payload[..]));
    // Reversed and with one fragment missing: never a wrong result.
    let mut a = Assembler::default();
    for f in frags.iter().rev().skip(1) {
        assert!(a.push(&f[4..]).is_none());
    }
    // Noise.
    let mut a = Assembler::default();
    for _ in 0..5000 {
        let junk: Vec<u8> = (0..gen_range(0, 40)).map(|_| gen_range(0u8, 255u8)).collect();
        let _ = a.push(&junk);
    }
    let mut r = Reader::new(&[1, 2]);
    let _ = (r.u32(), r.f64(), r.str(), r.u16());
    assert!(!r.ok);
}

#[test]
fn address_parsing_never_panics() {
    for s in ["", " ", ":", "::::", "999.999.999.999", "host:99999", "1.2.3.4:abc", "a".repeat(300).as_str(), "[::1]:7777", "127.0.0.1", "127.0.0.1:7777"] {
        let _ = net::resolve(s);
    }
    assert!(net::resolve("127.0.0.1").is_ok());
}

#[test]
fn records_and_achievements_survive_a_save_round_trip() {
    let mut s = crate::save::Save {
        achievements: Achievement::Fusionist.bit() | Achievement::Purist.bit(),
        challenges: Challenge::GlassCannon.bit(),
        best_kills: 12345,
        ..Default::default()
    };
    for i in 0..14u32 {
        let r = RunRecord {
            time: 60.0 * i as f32,
            kills: 100 * i,
            level: i,
            players: 1 + i % 4,
            won: i == 3,
            class: ALL_CLASSES[i as usize % ALL_CLASSES.len()],
            difficulty: "Abyss, cruel".into(),
            damage: 1000.0 * i as f64,
        };
        s.record_run(r);
    }
    assert_eq!(s.runs.len(), 10, "the leaderboard keeps ten");
    assert!(s.runs[0].won, "a won vigil ranks first");
    for pair in s.runs.windows(2) {
        assert!(!pair[1].better_than(&pair[0]), "leaderboard out of order");
    }
    let back = crate::save::Save::from_text(&s.to_text());
    assert_eq!(back.achievements, s.achievements);
    assert_eq!(back.challenges, s.challenges);
    assert_eq!(back.best_kills, s.best_kills);
    assert_eq!(back.runs.len(), 10);
    assert_eq!(back.runs[0].class, s.runs[0].class);
    assert_eq!(back.runs[0].kills, s.runs[0].kills);
    // Garbage lines are ignored, not fatal.
    let junk = crate::save::Save::from_text("run=1,2\nrun=a,b,c,d,e,f,g,h\nachievements=x\n");
    assert!(junk.runs.is_empty());
}

#[test]
fn challenges_change_the_hero() {
    let mut w = world_with(1, PRESETS[1].1);
    let normal_hp = w.heroes[0].stats.max_hp;
    w.heroes[0].challenges = Challenge::GlassCannon.bit() | Challenge::NoRolls.bit() | Challenge::OneWeapon.bit();
    w.heroes[0].recompute_stats();
    assert!(w.heroes[0].stats.max_hp < normal_hp * 0.6, "Glass Cannon halves health");
    let weapons = w.heroes[0].weapons.len();
    // Level up a lot; One Blade must never offer a new weapon.
    let peaks = simulate(&mut w, 240.0, true, |_, _| {});
    assert_bounded(&peaks);
    assert_eq!(w.heroes[0].weapons.len(), weapons, "One Blade picked up a new weapon");
    assert_eq!(w.heroes[0].rolls, 0, "Rooted still rolled");
    assert_eq!(challenge_mult(0), 1.0);
    assert_eq!(challenge_mult(0b111), 1.75);
}

#[test]
fn map_events_come_and_go() {
    let mut w = world_with(2, PRESETS[2].1);
    let mut saw_moon = false;
    let mut saw_curse = false;
    let mut saw_merchant = false;
    let mut kinds = [false; 32];
    let peaks = simulate(&mut w, 1200.0, true, |w, _| {
        saw_moon |= w.blood_moon > 0.0;
        saw_curse |= w.cursed.is_some();
        saw_merchant |= w.merchant.is_some();
        for e in &w.enemies {
            kinds[e.kind.to_u8() as usize] = true;
        }
    });
    println!("events: moons={} trials={} kills={} {:?}", w.moons_survived, w.trials_won, w.kills, peaks);
    assert_bounded(&peaks);
    assert!(saw_moon && saw_curse && saw_merchant, "moon {} curse {} merchant {}", saw_moon, saw_curse, saw_merchant);
    assert!(w.moons_survived >= 1, "the Blood Moon never ended");
    for k in [EnemyKind::Cultist, EnemyKind::Shieldbearer, EnemyKind::Necromancer, EnemyKind::Leech, EnemyKind::Banshee, EnemyKind::Gargoyle] {
        assert!(kinds[k.to_u8() as usize], "{:?} never spawned", k);
    }
}

/// Snapshot sizes over a long 4-player vigil. `cargo test --release snapshot_size -- --nocapture --ignored`
#[test]
#[ignore]
fn snapshot_size_report() {
    macroquad::rand::srand(7);
    let mut host = world_with(4, PRESETS[2].1);
    host.record_net = true;
    let (mut raw_sum, mut z_sum, mut n, mut z_max) = (0usize, 0usize, 0usize, 0usize);
    let mut enc_ms = 0.0;
    let mut clients: Vec<(snapshot::Encoder, snapshot::Decoder, World, Vec<u32>)> =
        (1..4).map(|_| (snapshot::Encoder::default(), snapshot::Decoder::default(), World::new(host.diff, None), Vec::new())).collect();
    let mut tick = 0;
    simulate(&mut host, 900.0, true, |w, f| {
        if f % 2 == 0 {
            tick += 1;
            let ev = snapshot::begin_round(w);
            for (slot, (enc, dec, cw, acks)) in clients.iter_mut().enumerate() {
                let slot = slot as u8 + 1;
                let ack = if acks.len() >= 2 { acks[acks.len() - 2] } else { 0 };
                let t = Instant::now();
                let z = enc.encode(w, &ev, &snapshot::View::of(w, slot, vec2(1920.0, 1080.0)), tick, ack);
                enc_ms += t.elapsed().as_secs_f64() * 1000.0;
                raw_sum += snapshot::last_raw_len();
                z_sum += z.len();
                z_max = z_max.max(z.len());
                n += 1;
                assert!(dec.decode(cw, &z, slot, 1.0 / 15.0));
                acks.push(dec.ack);
            }
        }
    });
    println!("snapshots {}: raw avg {} B, deflated avg {} B, max {} B, encode avg {:.3} ms", n, raw_sum / n, z_sum / n, z_max, enc_ms / n as f64);
}

/// Delta snapshots under heavy loss and late acknowledgements still land on exactly the host's state.
#[test]
fn delta_snapshots_stay_exact_under_loss() {
    macroquad::rand::srand(11);
    let mut host = world_with(2, PRESETS[2].1);
    host.record_net = true;
    let mut client = World::new(host.diff, None);
    client.is_client = true;
    let mut enc = snapshot::Encoder::default();
    let mut dec = snapshot::Decoder::default();
    // Acks travel back with a delay and can be lost too.
    let mut acks_in_flight: Vec<(u32, u32)> = Vec::new();
    let mut host_ack = 0;
    let (mut tick, mut applied, mut lost, mut checked) = (0u32, 0u32, 0u32, 0usize);
    simulate(&mut host, 400.0, true, |w, f| {
        if f % 2 != 0 {
            return;
        }
        tick += 1;
        acks_in_flight.retain(|&(arrive, ack)| {
            if arrive <= tick {
                host_ack = host_ack.max(ack);
                false
            } else {
                true
            }
        });
        let ev = snapshot::begin_round(w);
        let view = snapshot::View::of(w, 1, vec2(1600.0, 900.0));
        let data = enc.encode(w, &ev, &view, tick, host_ack);
        if gen_range(0.0, 1.0) < 0.3 {
            lost += 1;
            return;
        }
        assert!(dec.decode(&mut client, &data, 1, 1.0 / 15.0), "a snapshot coded against an acknowledged tick failed at {}", tick);
        applied += 1;
        if gen_range(0.0, 1.0) < 0.8 {
            acks_in_flight.push((tick + gen_range(1, 6), dec.ack));
        }
        // Every enemy the client has must sit exactly where the host has it, to the pixel.
        let by_id: HashMap<u16, Vec2> = w.enemies.iter().filter(|e| !e.dead).map(|e| (e.id as u16, e.pos)).collect();
        for ce in &client.enemies {
            let hp = by_id.get(&(ce.id as u16)).expect("client has an enemy the host does not");
            assert_eq!((ce.pos.x, ce.pos.y), (hp.x.round(), hp.y.round()), "enemy {} drifted", ce.id);
            checked += 1;
        }
        let picks: HashMap<u16, Vec2> = w.pickups.iter().filter(|p| p.id != 0).map(|p| (p.id as u16, p.pos)).collect();
        for cp in &client.pickups {
            let hp = picks.get(&(cp.id as u16)).expect("client has a pickup the host does not");
            assert_eq!((cp.pos.x, cp.pos.y), (hp.x.round(), hp.y.round()), "pickup {} drifted", cp.id);
        }
        let (hh, ch) = (&w.heroes[1], &client.heroes[1]);
        assert_eq!((hh.p.hp, hh.p.souls, hh.dmg_total as f32), (ch.p.hp, ch.p.souls, ch.dmg_total as f32));
        assert_eq!(hh.dmg_weapon.map(|d| d as f32), ch.dmg_weapon.map(|d| d as f32));
        assert_eq!(hh.weapons.len(), ch.weapons.len());
        assert_eq!(hh.stats.max_hp, ch.stats.max_hp);
        assert_eq!(hh.items.len(), ch.items.len());
    });
    println!("delta under loss: {} applied, {} lost, {} enemy positions checked", applied, lost, checked);
    assert!(applied > 3000 && checked > 100_000);
}

/// How long a mortal hero walking in circles survives per difficulty and class.
/// `cargo test --release balance -- --ignored --nocapture`
#[test]
#[ignore]
fn balance_report() {
    for (name, diff) in PRESETS.iter() {
        let mut results = Vec::new();
        for (k, class) in ALL_CLASSES.iter().enumerate() {
            macroquad::rand::srand(100 + k as u64);
            let mut w = World::new(*diff, None);
            w.screen = vec2(1920.0, 1080.0);
            let profile = Profile { unlocks: u64::MAX, ..Profile::default() };
            w.add_hero(0, "Bot", *class, profile, false);
            simulate(&mut w, VICTORY_TIME + 120.0, false, |_, _| {});
            results.push((class.name(), w.time as u32, w.level, w.bosses_killed, w.won));
        }
        let avg = results.iter().map(|r| r.1).sum::<u32>() / results.len() as u32;
        let wins = results.iter().filter(|r| r.4).count();
        println!("{:8} avg {:4}s ({:02}:{:02}), wins {}/{}", name, avg, avg / 60, avg % 60, wins, results.len());
        for r in &results {
            println!("    {:18} {:5}s  level {:3}  bosses {}", r.0, r.1, r.2, r.3);
        }
    }
}

#[test]
fn saves_survive_a_crash_mid_write() {
    let dir = std::env::temp_dir().join(format!("ashen_vigil_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let p = dir.join("ashen_vigil.sav");
    let mut s = crate::save::Save { souls: 111, ..Default::default() };
    s.store_at(&p).unwrap();
    s.souls = 222;
    s.store_at(&p).unwrap();
    assert_eq!(crate::save::Save::load_at(&p).souls, 222);
    // A crash that left the save truncated: the backup still has the previous state.
    std::fs::write(&p, "").unwrap();
    assert_eq!(crate::save::Save::load_at(&p).souls, 111);
    // A crash between the two renames: no save at all, only the backup.
    std::fs::remove_file(&p).unwrap();
    assert_eq!(crate::save::Save::load_at(&p).souls, 111);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_player_who_left_can_take_their_hero_back() {
    let mut w = world_with(3, PRESETS[1].1);
    simulate(&mut w, 60.0, true, |_, _| {});
    let weapons = w.heroes[2].weapons.len();
    w.disconnect(2);
    assert!(!w.heroes[2].connected && w.heroes[2].downed);
    assert!(!w.reconnect(1), "a connected hero cannot be reconnected");
    assert!(w.reconnect(2));
    let h = &w.heroes[2];
    assert!(h.connected && !h.downed && h.p.hp > 0.0);
    assert_eq!(h.name, "H2");
    assert_eq!(h.weapons.len(), weapons, "the returning hero keeps their build");
    simulate(&mut w, 30.0, true, |_, _| {});
}

#[test]
fn endless_night_goes_on_past_dawn() {
    let mut w = world_with(1, PRESETS[1].1);
    simulate(&mut w, VICTORY_TIME + 60.0, true, |_, _| {});
    // The test heroes walk in circles and can't fell the finale; it falls here instead.
    w.enemies.retain(|e| !e.boss);
    w.boss = None;
    w.boss_kind = None;
    w.arena = None;
    // No new lord may arrive in the meantime and hold the dawn back.
    w.next_boss = f32::MAX;
    simulate(&mut w, 30.0, true, |_, _| {});
    assert!(w.won, "the vigil should be kept at dawn");
    let bosses = w.bosses_spawned;
    w.won = false;
    w.endless = true;
    w.next_boss = w.time + 60.0;
    let peaks = simulate(&mut w, 900.0, true, |_, _| {});
    assert_bounded(&peaks);
    assert!(!w.won, "an endless vigil never ends in a win");
    assert!(w.time > VICTORY_TIME + 900.0, "the endless vigil stopped at {}", w.time);
    // A lord must come; whether these walking-in-circles test heroes fell it depends on
    // which lord it is.
    assert!(w.bosses_spawned > bosses, "no boss came after dawn");
    // Difficulty ramp: full strength only after three minutes.
    let d = PRESETS[3].1;
    assert!(d.at(0.0).hp < d.at(200.0).hp && d.at(200.0).hp == d.hp);
    assert_eq!(PRESETS[0].1.at(0.0).hp, PRESETS[0].1.hp, "easy settings are not ramped");
}

#[test]
fn packets_from_another_version_are_noticed() {
    let mut host = net::Sock::bind(0).unwrap();
    let port = host.local_port();
    let other = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    // Same game, older protocol byte, then a packet from something else entirely.
    other.send_to(&[0xA5, 0x1C, 5, 1, 0, 0], ("127.0.0.1", port)).unwrap();
    other.send_to(b"hello there", ("127.0.0.1", port)).unwrap();
    let start = Instant::now();
    while host.stats.foreign_version.is_none() && start.elapsed().as_secs_f32() < 2.0 {
        assert!(host.recv().is_none(), "a foreign packet was accepted");
    }
    assert_eq!(host.stats.foreign_version, Some(5));
}

/// Three hours of endless night with four heroes, then absurd numbers, all through snapshots.
/// Slow; `cargo test --release huge -- --ignored --nocapture`, ideally with overflow checks on.
#[test]
#[ignore]
fn huge_numbers_and_endless_hours() {
    let mut host = world_with(4, PRESETS[3].1);
    host.record_net = true;
    host.endless = true;
    let mut client = World::new(host.diff, None);
    client.is_client = true;
    let mut enc = snapshot::Encoder::default();
    let mut dec = snapshot::Decoder::default();
    let mut tick = 0;
    let mut max_bytes = 0;
    let peaks = simulate(&mut host, 3.0 * 3600.0, true, |w, f| {
        if f % 30 == 0 {
            tick += 1;
            let ev = snapshot::begin_round(w);
            let data = enc.encode(w, &ev, &snapshot::View::of(w, 1, vec2(1920.0, 1080.0)), tick, dec.ack);
            max_bytes = max_bytes.max(data.len());
            assert!(dec.decode(&mut client, &data, 1, 1.0), "decode failed at t {}", w.time);
        }
        if f % 9000 == 0 {
            for e in &w.enemies {
                assert!(e.hp.is_finite() && e.pos.is_finite() && e.max_hp.is_finite(), "enemy numbers broke at t {}: {:?}", w.time, (e.hp, e.max_hp));
            }
            for h in &w.heroes {
                assert!(h.stats.max_hp.is_finite() && h.dmg_total.is_finite(), "hero numbers broke at t {}", w.time);
            }
        }
    });
    println!(
        "3 h endless: t={:.0}s level={} kills={} bosses={} souls={} max snapshot {} B {:?}",
        host.time, host.level, host.kills, host.bosses_killed, host.heroes[1].p.souls, max_bytes, peaks
    );
    assert_bounded(&peaks);
    // Level-up pauses use up frames without game time, so a little less than three hours pass.
    assert!(host.time >= 2.5 * 3600.0, "the vigil stopped at {}", host.time);

    // Absurd values must arrive as they are, not cut off.
    let h = &mut host.heroes[1];
    h.p.souls = 9_000_000_000;
    h.banked = 7_000_000_000;
    h.dmg_total = 3.0e12;
    host.kills = u32::MAX - 5;
    host.net_nums.push(DmgNum { pos: host.heroes[1].p.pos, val: 250_000.0, life: 0.7, crit: true });
    tick += 1;
    let ev = snapshot::begin_round(&mut host);
    let data = enc.encode(&host, &ev, &snapshot::View::of(&host, 1, vec2(1920.0, 1080.0)), tick, dec.ack);
    client.show_numbers = true;
    client.nums.clear();
    assert!(dec.decode(&mut client, &data, 1, 1.0));
    let ch = client.heroes.iter().find(|h| h.slot == 1).unwrap();
    assert_eq!(ch.p.souls, 9_000_000_000, "carried souls were cut off");
    assert_eq!(ch.banked, 7_000_000_000, "banked souls were cut off");
    assert!((ch.dmg_total - 3.0e12).abs() / 3.0e12 < 1e-6, "damage total was cut off: {}", ch.dmg_total);
    assert_eq!(client.kills, u32::MAX - 5);
    assert!(client.nums.iter().any(|n| n.val >= 250_000.0), "a big hit showed a capped number: {:?}", client.nums.iter().map(|n| n.val).collect::<Vec<_>>());
}

#[test]
fn projectile_weapons_aim_at_foes_and_fly_straight() {
    let mut w = world_with(1, PRESETS[1].1);
    w.enemies.clear();
    w.heroes[0].weapons = vec![Weapon::new(WeaponKind::Daggers)];
    w.heroes[0].p.facing = vec2(0.0, -1.0);
    let hero = w.heroes[0].p.pos;
    let target = hero + vec2(300.0, 120.0);
    let e = w.make_enemy(EnemyKind::Knight, target);
    w.enemies.push(e);
    w.grid.rebuild(&w.enemies);
    w.heroes[0].weapons[0].timers[0] = 0.0;
    w.update_weapons(1.0 / 60.0);
    let p = w.projs.first().expect("the daggers did not fire");
    let want = (target - hero).normalize();
    assert!(p.vel.normalize().dot(want) > 0.99, "fired {:?}, the foe is at {:?}", p.vel.normalize(), want);
    // Once fired, a dagger keeps its course: no homing.
    let before = p.vel;
    w.enemies[0].pos = hero + vec2(-300.0, 0.0);
    w.grid.rebuild(&w.enemies);
    w.update_projs(1.0 / 60.0);
    assert_eq!(w.projs[0].vel, before);
}

#[test]
fn fusions_grow_into_mega_and_ultra_fusions() {
    let mut w = world_with(1, PRESETS[1].1);
    let maxed = |k: WeaponKind| {
        let mut wp = Weapon::new(k);
        wp.level = WEAPON_MAX_LEVEL;
        wp
    };
    // A new pair: Blade and Scythe.
    w.heroes[0].weapons = vec![maxed(WeaponKind::Blade), maxed(WeaponKind::Scythe)];
    w.check_fusion(0);
    assert_eq!(w.heroes[0].weapons.len(), 1);
    assert_eq!(w.heroes[0].weapons[0].display_name(), "Reaper's Waltz");
    // Two more pairs: the second fusion merges with the first into a mega fusion.
    w.heroes[0].weapons.push(maxed(WeaponKind::Orb));
    w.heroes[0].weapons.push(maxed(WeaponKind::Hex));
    w.check_fusion(0);
    assert_eq!(w.heroes[0].weapons.len(), 1);
    assert_eq!(w.heroes[0].weapons[0].tier(), 2, "two fusions should make a mega fusion");
    assert_eq!(w.heroes[0].weapons[0].parts().len(), 4);
    // Four more: another mega fusion, and the two megas become an ultra fusion.
    for k in [WeaponKind::Spear, WeaponKind::Bolt, WeaponKind::Chime, WeaponKind::Lanterns] {
        w.heroes[0].weapons.push(maxed(k));
    }
    w.check_fusion(0);
    let ultra = &w.heroes[0].weapons[0];
    assert_eq!(w.heroes[0].weapons.len(), 1);
    assert_eq!(ultra.tier(), 3);
    assert_eq!(ultra.parts().len(), 8);
    assert!(ultra.parts().iter().all(|p| p.mult == ULTRA_MULT && p.evo));
    assert_eq!(ultra.timers.len(), 8);
    // It fires every part and survives a long fight and a snapshot round trip.
    let peaks = simulate(&mut w, 60.0, true, |_, _| {});
    assert_bounded(&peaks);
    let mut enc = snapshot::Encoder::default();
    let ev = snapshot::begin_round(&mut w);
    let data = enc.encode(&w, &ev, &snapshot::View::of(&w, 0, vec2(1920.0, 1080.0)), 1, 0);
    let mut c = World::new(w.diff, None);
    c.is_client = true;
    assert!(snapshot::Decoder::default().decode(&mut c, &data, 0, 0.03));
    assert_eq!(c.heroes[0].weapons[0].kinds(), w.heroes[0].weapons[0].kinds());
}

#[test]
fn enemy_bullets_toughen_as_foes_do() {
    let mut w = world_with(1, PRESETS[1].1);
    w.enemy_bullet(vec2(0.0, 0.0), vec2(1.0, 0.0), 6.0, 5.0, 0.0, 3.0);
    let early = w.ebullets[0].hp;
    w.time = 25.0 * 60.0;
    w.enemy_bullet(vec2(0.0, 0.0), vec2(1.0, 0.0), 6.0, 5.0, 0.0, 3.0);
    let late = w.ebullets[1].hp;
    assert!(late > early * 5.0, "early {} late {}", early, late);
    // A weak hit only dents a tough bullet; enough damage breaks it.
    w.ebullets.remove(0);
    assert_eq!(w.damage_bullets(vec2(0.0, 0.0), 10.0, late * 0.5), 0);
    assert_eq!(w.ebullets.len(), 1);
    w.ebullets[0].hit_cd = 0.0;
    assert_eq!(w.damage_bullets(vec2(0.0, 0.0), 10.0, late), 1);
    assert!(w.ebullets.is_empty());
}

#[test]
fn the_lost_grave_gives_a_relic_once() {
    let mut w = world_with(1, PRESETS[1].1);
    assert!(inside_world(LOST_GRAVE, 100.0));
    w.heroes[0].p.pos = LOST_GRAVE + vec2(10.0, 0.0);
    w.update_secrets(0.016);
    assert!(w.grave_found);
    let banner = w.banner.clone().map(|b| b.0);
    w.banner = None;
    w.update_secrets(0.016);
    assert!(w.banner.is_none(), "the grave gave twice");
    assert_eq!(banner.as_deref(), Some("HERE LIES THE LAST KEEPER"));
}

#[test]
fn received_logs_are_kept_with_safe_names() {
    let dir = std::env::temp_dir().join(format!("ashen_vigil_recv_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for i in 0..25 {
        crate::log::store_received(&dir, &format!("We/ird:{}", i), "vigil", "text").unwrap();
    }
    let names: Vec<String> = std::fs::read_dir(dir.join("players")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert!(names.len() <= 20, "{}", names.len());
    assert!(names.iter().all(|n| !n.contains('/') && !n.contains(':')), "{:?}", names);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_grave_cat_waits_inside_the_walls() {
    for seed in 0..20 {
        macroquad::rand::srand(seed);
        let mut w = world_with(1, PRESETS[1].1);
        let cat = w.cat.as_ref().expect("no Grave Cat on the field").pos;
        assert!(inside_world(cat, 100.0), "the cat sits outside the walls at {:?}", cat);
        w.heroes[0].p.pos = cat + vec2(10.0, 0.0);
        w.update_cat(0.016);
        assert_eq!(w.cat.as_ref().unwrap().owner, Some(0), "walking up to the cat does not befriend it");
        assert!(w.heroes[0].has_cat);
    }
}

#[test]
fn log_file_names_get_the_date_right() {
    assert_eq!(crate::log::date_of(0), (1970, 1, 1));
    assert_eq!(crate::log::date_of(20_760), (2026, 11, 3));
    assert_eq!(crate::log::date_of(11_016), (2000, 2, 29));
}

#[test]
fn logs_keep_the_newest_sessions() {
    let dir = std::env::temp_dir().join(format!("ashen_vigil_logs_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..12 {
        std::fs::write(dir.join(format!("2026-01-{:02}_10-00-00.log", i + 1)), "old").unwrap();
    }
    std::fs::write(dir.join("notes.txt"), "not a log").unwrap();
    crate::log::init(&dir, None);
    crate::log!("hello from the test");
    let mut logs: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".log")).collect();
    logs.sort();
    assert_eq!(logs.len(), 10, "{:?}", logs);
    assert!(!logs.contains(&"2026-01-01_10-00-00.log".to_string()), "the oldest log should be gone");
    assert!(dir.join("notes.txt").exists(), "other files stay");
    let newest = std::fs::read_to_string(dir.join(logs.last().unwrap())).unwrap();
    assert!(newest.contains("hello from the test"), "{}", newest);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Hiss and buzz in each music loop; lower is calmer. `cargo test --release music_report -- --ignored --nocapture`
#[test]
#[ignore]
fn music_report() {
    for (name, r) in crate::audio::music_roughness() {
        println!("{:8} roughness {:6.2}", name, r);
    }
}

#[test]
fn sounds_fade_with_distance() {
    let w = world_with(1, PRESETS[1].1);
    let me = w.heroes[0].p.pos;
    assert_eq!(w.hearing(None), 1.0, "announcements are heard everywhere");
    assert_eq!(w.hearing(Some(me + vec2(100.0, 0.0))), 1.0);
    let mid = w.hearing(Some(me + vec2((HEAR_FULL + HEAR_MAX) * 0.5, 0.0)));
    assert!(mid > 0.3 && mid < 0.7, "{}", mid);
    assert_eq!(w.hearing(Some(me + vec2(HEAR_MAX + 50.0, 0.0))), 0.0, "far away is silent");
}

#[test]
fn full_floor_merges_old_orbs_and_new_ones_still_drop() {
    let mut w = world_with(1, PRESETS[1].1);
    w.pickups.clear();
    let me = w.heroes[0].p.pos;
    // A floor covered in single souls, in clumps far from the hero.
    for i in 0..MAX_PICKUPS {
        let at = me + vec2(800.0 + (i % 25) as f32 * 30.0, -600.0 + (i / 25) as f32 * 60.0);
        w.pickups.push(Pickup::new(at, PickKind::Soul(1.0)));
    }
    let total = |w: &World| w.pickups.iter().map(|p| if let PickKind::Soul(v) = p.kind { v as f64 } else { 0.0 }).sum::<f64>();
    let before = total(&w);
    let kill_at = me + vec2(40.0, 0.0);
    w.drop_soul(kill_at, 5.0);
    assert!(w.pickups.len() <= MERGED_PICKUPS + 1, "the floor did not make room: {}", w.pickups.len());
    assert!((total(&w) - (before + 5.0)).abs() < 1e-3, "souls were lost or made up: {} vs {}", total(&w), before + 5.0);
    let fresh = w.pickups.iter().any(|p| p.pos.distance(kill_at) < 1.0 && matches!(p.kind, PickKind::Soul(v) if (v - 5.0).abs() < 1e-3));
    assert!(fresh, "the new orb did not drop where the foe fell");
    assert!(w.pickups.iter().any(|p| matches!(p.kind, PickKind::Soul(v) if v > 1.5)), "no orb grew from merging");
    // Orbs already flying to a hero are left alone.
    for p in w.pickups.iter_mut() {
        p.pulled = Some(0);
    }
    let n = w.pickups.len();
    w.merge_souls();
    assert_eq!(w.pickups.len(), n);
}

/// Upgrades a hero until no level-up has anything left to offer.
fn max_out(w: &mut World, h: usize) {
    for _ in 0..500 {
        let Some(c) = w.heroes[h].pool(true).first().map(|p| p.0) else { break };
        w.heroes[h].apply_choice(c);
        w.check_fusion(h);
    }
    assert!(w.heroes[h].maxed(), "the hero could not be maxed out");
}

#[test]
fn maxed_heroes_take_souls_without_a_popup() {
    let mut w = world_with(1, PRESETS[1].1);
    max_out(&mut w, 0);
    let souls = w.heroes[0].p.souls;
    w.pending_levels = 3;
    for _ in 0..10 {
        w.update(1.0 / 60.0);
        assert_ne!(w.pause, Pause::Choosing, "a maxed hero got a level-up popup");
        w.heroes[0].p.hp = w.heroes[0].stats.max_hp;
        w.heroes[0].p.iframes = 1.0;
    }
    assert_eq!(w.pending_levels, 0);
    assert_eq!(w.heroes[0].p.souls - souls, 3 * 150, "each level-up should pay 150 souls");

    // Two players, one maxed: only the other one chooses, then the countdown follows.
    let mut w = world_with(2, PRESETS[1].1);
    max_out(&mut w, 0);
    w.pending_levels = 1;
    w.update(1.0 / 60.0);
    assert_eq!(w.pause, Pause::Choosing);
    assert!(w.heroes[0].picked && !w.heroes[1].picked);
    let round = w.level_round;
    w.pick(1, round, 0);
    w.update(1.0 / 60.0);
    assert!(matches!(w.pause, Pause::Countdown(_)), "{:?}", w.pause);
    assert_eq!(w.pending_levels, 0);
}

#[test]
fn the_countdown_shrinks_every_five_levels() {
    let mut w = world_with(2, PRESETS[1].1);
    w.level = 1;
    assert_eq!(w.resume_delay(), RESUME_DELAY);
    w.level = 5;
    assert!((w.resume_delay() - RESUME_DELAY * 0.95).abs() < 1e-5);
    w.level = 10;
    assert!((w.resume_delay() - RESUME_DELAY * 0.95 * 0.95).abs() < 1e-5);
    w.level = 400;
    assert_eq!(w.resume_delay(), MIN_RESUME_DELAY);
}

#[test]
fn bosses_grow_with_time_and_take_turns_past_dawn() {
    let mut w = world_with(1, PRESETS[1].1);
    w.time = 300.0;
    w.next_boss = 300.0;
    w.update_spawns(0.0);
    let first = w.enemies.iter().find(|e| e.boss).map(|e| e.max_hp).unwrap();
    w.enemies.retain(|e| !e.boss);
    w.boss = None;
    w.time = 25.0 * 60.0;
    w.next_boss = w.time;
    w.update_spawns(0.0);
    let late = w.enemies.iter().find(|e| e.boss).map(|e| (e.max_hp, e.kind)).unwrap();
    assert!(late.0 > first * 20.0, "a boss at 25:00 should be far tougher: {} vs {}", late.0, first);
    // Past dawn the lords take turns instead of the Sovereign every time.
    w.enemies.retain(|e| !e.boss);
    w.boss = None;
    w.endless = true;
    let mut kinds = Vec::new();
    for k in 0..4 {
        w.time = VICTORY_TIME + 60.0 + k as f32 * 300.0;
        w.next_boss = w.time;
        w.update_spawns(0.0);
        kinds.push(w.boss_kind);
        w.enemies.retain(|e| !e.boss);
        w.boss = None;
    }
    kinds.sort_by_key(|k| k.map(|k| k as u8));
    kinds.dedup();
    assert_eq!(kinds.len(), 4, "every lord should come past dawn: {:?}", kinds);
}

#[test]
fn the_permanent_shop_feeds_the_stats() {
    let mut profile = Profile { unlocks: u64::MAX, ..Profile::default() };
    let base = Hero::new(0, "A", Class::Wanderer, profile).stats;
    for (i, m) in META.iter().enumerate() {
        profile.meta[i] = m.max;
    }
    for (i, r) in RELICS.iter().enumerate() {
        profile.relics[i] = r.max;
    }
    let full = Hero::new(0, "A", Class::Wanderer, profile).stats;
    assert!(full.armor >= base.armor + 7.0);
    assert!(full.regen > base.regen + 0.7);
    assert_eq!(full.amount, base.amount + 2);
    assert!(full.area > base.area && full.crit > base.crit && full.growth > base.growth && full.elite_dmg > base.elite_dmg);
    assert!(full.cooldown >= 0.4);
    // Prices climb: the last level of every upgrade costs more than the first.
    for (i, m) in META.iter().enumerate() {
        assert!(meta_cost(i, m.max - 1) >= meta_cost(i, 0));
    }
    assert!(ALL_ACHIEVEMENTS.len() <= 128, "achievements are bits of a u128");
    let mut bits: Vec<u128> = ALL_ACHIEVEMENTS.iter().map(|a| a.bit()).collect();
    bits.sort_unstable();
    bits.dedup();
    assert_eq!(bits.len(), ALL_ACHIEVEMENTS.len());
}

#[test]
fn level_up_cards_point_out_fusions_and_evolutions() {
    let mut w = world_with(1, PRESETS[1].1);
    w.heroes[0].weapons = vec![Weapon::new(WeaponKind::Blade)];
    w.heroes[0].items.clear();
    let h = &w.heroes[0];
    assert!(h.choice_hint(Choice::NewWeapon(WeaponKind::Axe)).is_some_and(|(t, _)| t.contains("Rusted Blade")));
    assert!(h.choice_hint(Choice::NewWeapon(WeaponKind::Scythe)).is_some(), "the new second partner counts too");
    assert!(h.choice_hint(Choice::NewWeapon(WeaponKind::Orb)).is_none());
    let evo = WeaponKind::Blade.evo_item();
    assert!(h.choice_hint(Choice::NewItem(evo)).is_some_and(|(t, _)| t.starts_with("Evolves")));
}

#[test]
fn cute_mode_softens_the_red() {
    let mut w = world_with(1, PRESETS[1].1);
    let blood = Color::new(0.6, 0.05, 0.05, 1.0);
    assert_eq!(w.soften(blood), blood);
    w.cute = true;
    let soft = w.soften(blood);
    assert!(soft.b > 0.5 && soft.r > 0.9, "{:?}", soft);
    let teal = Color::new(0.2, 0.9, 0.8, 1.0);
    assert_eq!(w.soften(teal), teal);
}

#[test]
fn every_weapon_ends_as_the_last_flame() {
    let mut w = world_with(1, PRESETS[1].1);
    w.heroes[0].weapons = ALL_WEAPONS
        .iter()
        .map(|k| {
            let mut wp = Weapon::new(*k);
            wp.level = WEAPON_MAX_LEVEL;
            wp
        })
        .collect();
    w.check_fusion(0);
    let h = &w.heroes[0];
    assert_eq!(h.weapons.len(), 1, "every weapon should have gone into one slot");
    let last = &h.weapons[0];
    assert!(last.whole && last.tier() == 4);
    assert_eq!(last.parts().len(), ALL_WEAPONS.len());
    assert!(last.parts().iter().all(|p| p.mult == LAST_MULT));
    assert_eq!(last.display_name(), LAST_FLAME);

    // While a new weapon can still come, leftovers wait instead of merging.
    let mut w = world_with(1, PRESETS[1].1);
    let mut wp = Weapon::new(WeaponKind::Orb);
    wp.level = WEAPON_MAX_LEVEL;
    w.heroes[0].weapons = vec![wp.clone(), Weapon { kind: WeaponKind::Chime, ..wp }];
    w.check_fusion(0);
    assert_eq!(w.heroes[0].weapons.len(), 2);
}

#[test]
fn the_new_shrine_upgrades_take_effect() {
    let base = Hero::new(0, "A", Class::Wanderer, Profile::default());
    let mut profile = Profile::default();
    for (i, m) in META.iter().enumerate() {
        profile.meta[i] = m.max;
    }
    let full = Hero::new(0, "A", Class::Wanderer, profile);
    assert!(full.stats.max_hp >= base.stats.max_hp + 150.0);
    assert!(full.stats.lifesteal > base.stats.lifesteal);
    assert!(full.roll_cost() < base.roll_cost());
    assert_eq!(full.max_weapons(), MAX_WEAPONS + 2);
    assert_eq!(full.max_items(), MAX_ITEMS + 2);
    assert_eq!(full.weapons[0].level, 1 + META[M_KINDLING].max);
    assert!(full.draught_mult() > 1.9);
    assert!(full.rise_health(0.4) > base.rise_health(0.4));
    let mut wp = Weapon::new(WeaponKind::Blade);
    wp.level = WEAPON_MAX_LEVEL;
    assert!(full.weapon_bonus(&wp) > 1.3 && base.weapon_bonus(&wp) == 1.0);
}

#[test]
fn skins_travel_to_clients_and_survive_the_save() {
    let mut host = world_with(2, PRESETS[1].1);
    host.heroes[1].skin = 7;
    let mut client = World::new(host.diff, None);
    client.is_client = true;
    let mut enc = snapshot::Encoder::default();
    let mut dec = snapshot::Decoder::default();
    let ev = snapshot::begin_round(&mut host);
    let view = snapshot::View::of(&host, 0, vec2(1920.0, 1080.0));
    let data = enc.encode(&host, &ev, &view, 1, 0);
    assert!(dec.decode(&mut client, &data, 0, 0.0));
    let h = client.heroes.iter().find(|h| h.slot == 1).expect("hero 1 arrived");
    assert_eq!(h.skin, 7);

    let mut save = Save { skins: 1 << 6, marks: 3, ..Default::default() };
    save.profile.skin = 7;
    let back = Save::from_text(&save.to_text());
    assert_eq!((back.skins, back.profile.skin), (1 << 6, 7));
    // A skin that is not owned is not worn.
    save.skins = 0;
    assert_eq!(Save::from_text(&save.to_text()).profile.skin, 0);
}

#[test]
fn a_lord_is_announced_five_seconds_ahead() {
    let mut w = world_with(1, PRESETS[1].1);
    w.time = BOSS_INTERVAL - 8.0;
    w.next_boss = BOSS_INTERVAL;
    let mut warned_at = None;
    simulate(&mut w, 10.0, true, |w, _| {
        if w.boss_warning > 0.0 && warned_at.is_none() {
            warned_at = Some(w.next_boss - w.time);
        }
    });
    let ahead = warned_at.expect("no warning before the lord");
    assert!((4.9..=5.01).contains(&ahead), "warned {} s ahead", ahead);
    assert!(w.boss.is_some(), "the lord did not come");
    assert_eq!(w.boss_warning, 0.0, "the warning stays up during the fight");
    // A lord that outlives its five minutes still leaves the next one its warning.
    let i = w.enemies.iter().position(|e| e.boss).unwrap();
    w.next_boss = w.time - 1.0;
    let src = Src { hero: 0, weapon: WeaponKind::Blade };
    w.enemies[i].hp = 1.0;
    w.hit(i, 1e12, Vec2::ZERO, src, false);
    assert!(w.next_boss >= w.time + BOSS_WARNING - 0.01);
}

#[test]
fn a_boss_rush_brings_lord_after_lord_and_goes_on_endless() {
    let mut w = world_with(1, PRESETS[1].1);
    w.start_boss_rush();
    let src = Src { hero: 0, weapon: WeaponKind::Blade };
    let mut warned = false;
    let mut lords = Vec::new();
    for _ in 0..RUSH_LORDS {
        simulate(&mut w, RUSH_BREATHER + 1.0, true, |w, _| warned |= w.boss_warning > 0.0);
        let i = w.enemies.iter().position(|e| e.boss).expect("the next lord did not come");
        lords.push(w.enemies[i].max_hp);
        w.enemies[i].hp = 1.0;
        w.hit(i, 1e15, Vec2::ZERO, src, false);
    }
    simulate(&mut w, 1.0, true, |_, _| {});
    assert!(!warned, "a boss rush gives no warning");
    assert!(w.won, "twelve lords should end the boss rush");
    assert!(w.time < 20.0 * 60.0, "the rush took {} s", w.time);
    assert!(lords.windows(2).all(|p| p[1] > p[0]), "every lord should be stronger than the last: {:?}", lords);
    assert!(lords[RUSH_LORDS as usize - 1] > lords[0] * 5.0, "late lords should be far stronger: {:?}", lords);
    // The endless rush: more lords, no end.
    w.won = false;
    w.endless = true;
    w.endless_since = w.time;
    w.next_boss = w.time + RUSH_BREATHER;
    for _ in 0..3 {
        simulate(&mut w, RUSH_BREATHER + 1.0, true, |_, _| {});
        let i = w.enemies.iter().position(|e| e.boss).expect("the endless rush stopped sending lords");
        w.enemies[i].hp = 1.0;
        w.hit(i, 1e15, Vec2::ZERO, src, false);
    }
    simulate(&mut w, 1.0, true, |_, _| {});
    assert!(!w.won && w.bosses_killed == RUSH_LORDS + 3);
}

#[test]
fn clients_learn_the_mode_the_warning_and_the_challenges() {
    let mut host = world_with(2, PRESETS[1].1);
    host.boss_rush = true;
    host.boss_warning = 3.2;
    host.heroes[1].challenges = Challenge::NoRolls.bit() | Challenge::GlassCannon.bit();
    let mut client = World::new(host.diff, None);
    client.is_client = true;
    let mut enc = snapshot::Encoder::default();
    let mut dec = snapshot::Decoder::default();
    let ev = snapshot::begin_round(&mut host);
    let view = snapshot::View::of(&host, 1, vec2(1920.0, 1080.0));
    let data = enc.encode(&host, &ev, &view, 1, 0);
    assert!(dec.decode(&mut client, &data, 1, 0.0));
    assert!(client.boss_rush);
    assert!((client.boss_warning - 3.2).abs() < 0.11);
    let me = client.heroes.iter().find(|h| h.slot == 1).unwrap();
    assert_eq!(me.challenges, Challenge::NoRolls.bit() | Challenge::GlassCannon.bit());
}

#[test]
#[ignore]
fn boss_rush_report() {
    for heroes in [1, 4] {
        macroquad::rand::srand(5);
        let mut w = world_with(heroes, PRESETS[1].1);
        w.start_boss_rush();
        let mut fights: Vec<(u32, f32)> = Vec::new();
        let mut started = None;
        let peaks = simulate(&mut w, 1500.0, true, |w, _| match (w.boss.is_some(), started) {
            (true, None) => started = Some(w.time),
            (false, Some(t)) => {
                fights.push((w.level, w.time - t));
                started = None;
            }
            _ => {}
        });
        assert_bounded(&peaks);
        println!("{} heroes: {} lords down by {:.0}s, won {}; level and fight seconds: {:?}", heroes, w.bosses_killed, w.time, w.won, fights);
    }
}

/// Everything that must hold in every frame, whatever happens.
fn check_invariants(w: &World, ctx: &str) {
    let ok = |v: f32| v.is_finite();
    for e in &w.enemies {
        assert!(ok(e.pos.x) && ok(e.pos.y) && ok(e.hp) && ok(e.max_hp) && ok(e.speed), "{}: bad enemy {:?} at {:?} hp {}", ctx, e.kind, e.pos, e.hp);
        assert!(inside_world(e.pos, -400.0), "{}: enemy left the world at {:?}", ctx, e.pos);
    }
    for h in &w.heroes {
        assert!(ok(h.p.pos.x) && ok(h.p.pos.y) && ok(h.p.hp) && ok(h.p.stamina), "{}: bad hero state", ctx);
        assert!(h.p.hp <= h.stats.max_hp + 0.01, "{}: hp {} over max {}", ctx, h.p.hp, h.stats.max_hp);
        assert!(h.stats.to_array().iter().all(|v| ok(*v)), "{}: bad stats {:?}", ctx, h.stats);
        assert!(h.weapons.len() <= h.max_weapons(), "{}: {} weapons, room for {}", ctx, h.weapons.len(), h.max_weapons());
        assert!(h.items.len() <= h.max_items(), "{}: {} items, room for {}", ctx, h.items.len(), h.max_items());
        let mut kinds: Vec<usize> = h.weapons.iter().flat_map(|wp| wp.kinds()).map(|k| k.idx()).collect();
        let n = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), n, "{}: a weapon is carried twice: {:?}", ctx, h.weapons.iter().map(|w| w.kinds()).collect::<Vec<_>>());
        for wp in &h.weapons {
            assert_eq!(wp.timers.len(), wp.fused.len() + 1, "{}: timers out of step", ctx);
            assert_eq!(wp.spins.len(), wp.fused.len() + 1, "{}: spins out of step", ctx);
            assert!(wp.level <= WEAPON_MAX_LEVEL || wp.is_fused(), "{}: weapon level {}", ctx, wp.level);
        }
        for it in &h.items {
            assert!(it.level >= 1 && it.level <= it.kind.max_level(), "{}: item level {}", ctx, it.level);
        }
        let mut items: Vec<usize> = h.items.iter().map(|i| i.kind.idx()).collect();
        let n = items.len();
        items.sort_unstable();
        items.dedup();
        assert_eq!(items.len(), n, "{}: an item is carried twice", ctx);
    }
    for p in &w.pickups {
        assert!(ok(p.pos.x) && ok(p.pos.y), "{}: bad pickup", ctx);
    }
    for b in &w.ebullets {
        assert!(ok(b.pos.x) && ok(b.pos.y) && ok(b.vel.x), "{}: bad bullet", ctx);
    }
    assert!(ok(w.xp) && ok(w.xp_next) && w.xp_next > 0.0, "{}: bad xp {} / {}", ctx, w.xp, w.xp_next);
    assert!(w.boss_warning >= 0.0 && w.boss_warning <= BOSS_WARNING + 0.01, "{}: warning {}", ctx, w.boss_warning);
}

#[test]
#[ignore]
fn chaos_across_modes() {
    let secs: f32 = std::env::var("ASHEN_CHAOS_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(900.0);
    for seed in 1..=std::env::var("ASHEN_CHAOS_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(8u64) {
        macroquad::rand::srand(seed * 7919);
        let heroes = 1 + (seed as usize % 4);
        let diff = PRESETS[seed as usize % PRESETS.len()].1;
        let mut w = World::new(diff, None);
        w.screen = vec2(1920.0, 1080.0);
        w.record_net = heroes > 1;
        for i in 0..heroes {
            let mut profile = Profile { unlocks: if seed % 3 == 0 { default_unlocks() } else { u64::MAX }, ..Profile::default() };
            for (k, m) in META.iter().enumerate() {
                profile.meta[k] = gen_range(0, m.max + 1);
            }
            for (k, r) in RELICS.iter().enumerate() {
                profile.relics[k] = gen_range(0, r.max + 1);
            }
            profile.skin = gen_range(0, SKINS.len() as u8 + 1);
            let h = w.add_hero(i as u8, &format!("T{}", i), ALL_CLASSES[(seed as usize + i) % ALL_CLASSES.len()], profile, false);
            w.heroes[h].challenges = (seed % 8) as u8;
            w.heroes[h].recompute_stats();
            w.heroes[h].p.hp = w.heroes[h].stats.max_hp;
        }
        w.grant_head_start();
        let rush = seed % 2 == 0;
        if rush {
            w.start_boss_rush();
        }
        let ctx = format!("seed {} heroes {} rush {} challenges {:03b}", seed, heroes, rush, seed % 8);
        let mut client = World::new(diff, None);
        client.is_client = true;
        let (mut enc, mut dec) = (snapshot::Encoder::default(), snapshot::Decoder::default());
        let mut tick = 0u32;
        let mut endless_done = false;
        for round in 0..2 {
            simulate(&mut w, secs, true, |w, f| {
                // Random cards and rerolls instead of always the first.
                if w.pause == Pause::Choosing && f % 3 == 0 {
                    for h in 0..w.heroes.len() {
                        if gen_range(0, 5) == 0 {
                            w.reroll(h);
                        }
                        let n = w.heroes[h].choices.len().max(1);
                        let round = w.level_round;
                        w.pick(h, round, gen_range(0, n));
                    }
                }
                check_invariants(w, &ctx);
                if f % 2 == 0 {
                    tick += 1;
                    let ev = snapshot::begin_round(w);
                    let view = snapshot::View::of(w, 0, vec2(1920.0, 1080.0));
                    let data = enc.encode(w, &ev, &view, tick, tick.saturating_sub(1));
                    assert!(dec.decode(&mut client, &data, 0, 1.0 / 15.0), "{}: client could not decode", ctx);
                    assert_eq!(client.heroes.len(), w.heroes.len(), "{}", ctx);
                }
            });
            // Kill the lord in the way so the run can reach its end, then go on endless once.
            if round == 0 && !w.won {
                w.enemies.retain(|e| !e.boss);
                w.boss = None;
                w.boss_kind = None;
                w.arena = None;
                if rush {
                    w.bosses_killed = w.bosses_killed.max(RUSH_LORDS);
                } else {
                    w.time = w.time.max(VICTORY_TIME);
                }
                w.next_boss = w.time + 30.0;
                simulate(&mut w, 2.0, true, |_, _| {});
            }
            if w.won && !endless_done {
                endless_done = true;
                w.won = false;
                w.endless = true;
                w.endless_since = w.time;
                w.next_boss = w.time + if rush { RUSH_BREATHER } else { 60.0 };
            }
        }
        println!("{}: reached {:.0}s, level {}, lords {}, endless {}", ctx, w.time, w.level, w.bosses_killed, w.endless);
    }
}

#[test]
#[ignore]
fn last_flame_perf_report() {
    macroquad::rand::srand(3);
    let mut w = world_with(4, PRESETS[3].1);
    for h in w.heroes.iter_mut() {
        let mut last = Weapon::with_parts(WeaponKind::Blade, ALL_WEAPONS[1..].to_vec());
        last.whole = true;
        last.evolved = true;
        last.level = WEAPON_MAX_LEVEL;
        h.weapons = vec![last];
        h.items = ALL_ITEMS.iter().take(8).map(|k| Item { kind: *k, level: k.max_level() }).collect();
        h.profile.meta[M_ARSENAL] = 2;
        h.profile.meta[M_SATCHEL] = 2;
        h.recompute_stats();
    }
    w.time = 25.0 * 60.0;
    let peaks = simulate(&mut w, 120.0, true, |w, _| {
        // Keep the horde as large as it may get.
        while w.enemies.len() < MAX_ENEMIES {
            let at = w.heroes[0].p.pos + rand_dir() * gen_range(300.0, 900.0);
            let e = w.make_enemy(EnemyKind::Ghoul, clamp_world(at, 40.0));
            w.enemies.push(e);
        }
    });
    assert_bounded(&peaks);
    println!("last flame x4, full horde: avg {:.2} ms, worst {:.2} ms per update; peaks {:?}", peaks.avg_update_ms, peaks.worst_update_ms, peaks);
}

#[test]
fn every_lord_fights_through_all_its_phases() {
    for (k, lord) in ALL_LORDS.iter().enumerate() {
        macroquad::rand::srand(100 + k as u64);
        let mut w = world_with(2, PRESETS[2].1);
        w.time = 5.0 * 60.0;
        w.next_boss = w.time;
        w.boss_rush = true;
        w.bosses_spawned = k as u32;
        simulate(&mut w, 1.0, true, |_, _| {});
        assert_eq!(w.boss_kind, Some(*lord), "the wrong lord came");
        assert!(w.quote.is_some(), "{:?} said nothing on arrival", lord);
        // Fight a while in each phase: full health, then half, then a fifth.
        // So much health that no single frame can fell it; the phases go by percentage.
        if let Some(e) = w.enemies.iter_mut().find(|e| e.boss) {
            e.max_hp = 1e12;
        }
        for frac in [1.0, 0.45, 0.2] {
            if let Some(e) = w.enemies.iter_mut().find(|e| e.boss) {
                e.hp = e.max_hp * frac;
            }
            // Keep it alive in this phase: the test heroes may hit hard.
            let peaks = simulate(&mut w, 25.0, true, |w, _| {
                if let Some(e) = w.enemies.iter_mut().find(|e| e.boss) {
                    e.hp = e.hp.max(e.max_hp * frac * 0.8);
                }
                check_invariants(w, "lord fight");
            });
            assert_bounded(&peaks);
        }
        let ai = w.boss.as_ref().expect("the lord vanished");
        assert!(ai.phase2, "{:?} never reached its second phase", lord);
        assert_eq!(ai.phase3, lord.has_third_phase(), "{:?} third phase", lord);
        assert!(lord.lines().iter().all(|l| l.len() < 70), "{:?} has a line too long for the screen", lord);
    }
}

#[test]
fn each_field_has_its_own_lords_and_foes() {
    for b in ALL_BIOMES {
        let lords = b.lords();
        assert!(!lords.contains(&BossKind::Sovereign), "the Sovereign belongs to the finale");
        let mut sorted: Vec<u8> = lords.iter().map(|k| *k as u8).collect();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 5, "{:?} repeats a lord", b);
    }
    // Every lord shows up on some field.
    for lord in ALL_LORDS.iter().filter(|l| **l != BossKind::Sovereign) {
        assert!(ALL_BIOMES.iter().any(|b| b.lords().contains(lord)), "{:?} never comes", lord);
    }
    let mut w = world_with(1, PRESETS[1].1);
    w.biome = Biome::Marsh;
    w.time = 5.0 * 60.0;
    w.next_boss = w.time;
    simulate(&mut w, 1.0, true, |_, _| {});
    assert_eq!(w.boss_kind, Some(Biome::Marsh.lords()[0]));
}

#[test]
fn every_class_skill_fires_and_cools_down() {
    for class in ALL_CLASSES {
        let mut w = World::new(PRESETS[1].1, None);
        w.screen = vec2(1920.0, 1080.0);
        w.add_hero(0, "S", class, Profile { unlocks: u64::MAX, ..Profile::default() }, false);
        for k in 0..30 {
            let e = w.make_enemy(EnemyKind::Hollow, Vec2::from_angle(k as f32 * 0.21) * (80.0 + k as f32 * 6.0));
            w.enemies.push(e);
        }
        w.heroes[0].input.skill = true;
        simulate(&mut w, 0.2, true, |w, _| w.heroes[0].input.skill = true);
        let cd = class.skill().2;
        assert!(w.heroes[0].skill_cd > cd * 0.3, "{:?}: no cooldown after the skill", class);
        // Pressing again does nothing until it is ready.
        let before = w.heroes[0].skill_cd;
        w.use_skill(0);
        simulate(&mut w, 0.1, true, |_, _| {});
        assert!(w.heroes[0].skill_cd < before, "{:?}: cast again while cooling down", class);
    }
}

#[test]
fn braziers_light_up_and_burn_out() {
    let mut w = world_with(1, PRESETS[1].1);
    assert!(w.braziers.len() >= 25, "only {} braziers", w.braziers.len());
    let at = w.braziers[0].pos;
    w.heroes[0].p.pos = at + vec2(20.0, 0.0);
    simulate(&mut w, BRAZIER_KINDLE + 0.3, true, |w, _| w.heroes[0].p.pos = at + vec2(20.0, 0.0));
    assert!(w.braziers[0].burn > 0.0, "standing by it did not light it");
    // It heals the hero standing by it...
    w.heroes[0].p.hp = 10.0;
    w.heroes[0].stats.regen = 0.0;
    simulate(&mut w, 2.0, false, |w, _| {
        w.enemies.clear();
        w.heroes[0].p.pos = at + vec2(20.0, 0.0);
    });
    assert!(w.heroes[0].p.hp > 12.0, "the brazier did not mend the hero: {}", w.heroes[0].p.hp);
    // ...and sets fire to a foe that comes close.
    let e = w.make_enemy(EnemyKind::Knight, at + vec2(80.0, 0.0));
    w.enemies.push(e);
    w.update_braziers(0.1);
    assert!(w.enemies.last().is_some_and(|e| e.burn > 0.0), "the foe did not catch fire");
    w.braziers[0].burn = 0.5;
    w.heroes[0].p.pos = at + vec2(500.0, 0.0);
    simulate(&mut w, 1.0, true, |_, _| {});
    assert_eq!(w.braziers[0].burn, 0.0, "it never burned out");
}

#[test]
fn curses_take_hold() {
    let mut calm = world_with(1, PRESETS[1].1);
    let mut cursed = world_with(1, PRESETS[1].1);
    cursed.lay_curses(Curse::Frenzy.bit() | Curse::DarkHour.bit() | Curse::Brittle.bit() | Curse::Famine.bit());
    let a = calm.make_enemy(EnemyKind::Hollow, Vec2::ZERO);
    let b = cursed.make_enemy(EnemyKind::Hollow, Vec2::ZERO);
    assert!(b.max_hp > a.max_hp * 1.1, "the Dark Hour should toughen foes");
    // Speeds vary a little per foe; compare the average of many.
    let avg = |w: &mut World| (0..50).map(|_| w.make_enemy(EnemyKind::Hollow, Vec2::ZERO).speed).sum::<f32>() / 50.0;
    let (calm_speed, cursed_speed) = (avg(&mut calm), avg(&mut cursed));
    assert!(cursed_speed > calm_speed * 1.15, "Frenzy should speed foes up: {} vs {}", cursed_speed, calm_speed);
    assert_eq!(cursed.heroes[0].p.revives, 0, "Brittle leaves no embers");
    // Famine: no regeneration.
    cursed.heroes[0].stats.regen = 5.0;
    cursed.heroes[0].p.hp = 10.0;
    cursed.enemies.clear();
    simulate(&mut cursed, 2.0, false, |w, _| w.enemies.clear());
    assert!(cursed.heroes[0].p.hp <= 10.0 + 0.01, "Famine still let the hero regenerate");
    assert!(curse_mult(cursed.curses) > 1.79);
}

#[test]
fn no_single_hit_takes_more_than_a_sliver_of_a_lord() {
    let mut w = world_with(1, PRESETS[1].1);
    w.time = 5.0 * 60.0;
    w.next_boss = w.time;
    simulate(&mut w, 0.5, true, |_, _| {});
    let i = w.enemies.iter().position(|e| e.boss).expect("no lord came");
    let max = w.enemies[i].max_hp;
    let before = w.enemies[i].hp;
    let src = Src { hero: 0, weapon: WeaponKind::Blade };
    w.hit(i, 1e15, Vec2::ZERO, src, false);
    let taken = before - w.enemies[i].hp;
    assert!(taken <= max * BOSS_HIT_CAP + 0.01, "one hit took {} of {}", taken, max);
    assert!(!w.enemies[i].dead);
    // Ordinary foes still fall to one big hit.
    let e = w.make_enemy(EnemyKind::Knight, vec2(300.0, 0.0));
    w.enemies.push(e);
    let j = w.enemies.len() - 1;
    w.hit(j, 1e15, Vec2::ZERO, src, false);
    assert!(w.enemies[j].dead);
}

#[test]
#[ignore]
#[cfg(windows)]
fn discord_pipe_report() {
    // The game's own id should get READY; a made-up one a clean error rather than a hang.
    println!("game id: {:?}", crate::discord::probe(crate::discord::APP_ID));
    println!("made-up id: {:?}", crate::discord::probe("123456789012345678"));
}

/// Voice trials: each kokoro_<lord>__<voice>.wav in VOICE_DIR, a line recorded with that
/// Kokoro voice, comes out as fx_<lord>__<voice>.wav in the lord's voice.
#[test]
#[ignore]
fn voice_samples() {
    let dir = std::env::var("VOICE_DIR").unwrap_or_else(|_| ".".into());
    for f in std::fs::read_dir(&dir).expect("VOICE_DIR").flatten() {
        let file = f.file_name().to_string_lossy().into_owned();
        let Some((lord, voice)) = file.strip_prefix("kokoro_").and_then(|v| v.strip_suffix(".wav")).and_then(|v| v.split_once("__")) else { continue };
        let Some(kind) = crate::world::ALL_LORDS.into_iter().find(|k| crate::voice::key(*k) == lord) else { continue };
        let rec = crate::voice::from_wav(&std::fs::read(f.path()).unwrap()).expect("a wav");
        let out = crate::voice::trim(&crate::voice::process_from(&rec, kind, crate::voice::recorded_pitch(voice)));
        std::fs::write(format!("{}/fx_{}__{}.wav", dir, lord, voice), crate::voice::to_wav(&out)).unwrap();
    }
}

/// Writes the game icon as a large PNG (for Discord's art assets): `ICON_OUT=path cargo test icon_png -- --ignored`.
#[test]
#[ignore]
fn icon_png() {
    let Ok(path) = std::env::var("ICON_OUT") else { return };
    let size = 1024;
    // export_png writes the rows bottom-up, so hand them over flipped.
    let rgba = crate::icon_art::icon_rgba(size);
    let bytes = rgba.chunks(size * 4).rev().flatten().copied().collect();
    let img = macroquad::texture::Image { width: size as u16, height: size as u16, bytes };
    img.export_png(&path);
}

/// Writes the lords' lines to record, one per row: file key, Kokoro voice, speed, text.
/// `VOICE_RAW=dir cargo test voice_jobs -- --ignored`, then record them as <key>.wav there.
#[test]
#[ignore]
fn voice_jobs() {
    let Ok(dir) = std::env::var("VOICE_RAW") else { return };
    let mut out = String::new();
    for kind in crate::world::ALL_LORDS {
        let Some((voice, speed)) = crate::voice::recorded_with(kind) else { continue };
        for (i, line) in kind.lines().iter().enumerate().filter(|(_, l)| !l.is_empty()) {
            out += &format!("{}_{}\t{}\t{}\t{}\n", crate::voice::key(kind), i, voice, speed, crate::voice::spoken(line));
        }
    }
    std::fs::write(format!("{}/jobs.tsv", dir), out).unwrap();
}

/// Gives the recorded lines in VOICE_RAW each lord's voice and writes them to VOICE_OUT;
/// the Hound's growls are synthesized.
#[test]
#[ignore]
fn voice_bake() {
    let (Ok(raw), Ok(out)) = (std::env::var("VOICE_RAW"), std::env::var("VOICE_OUT")) else { return };
    for kind in crate::world::ALL_LORDS {
        for (i, line) in kind.lines().iter().enumerate().filter(|(_, l)| !l.is_empty()) {
            let name = format!("{}_{}", crate::voice::key(kind), i);
            let audio = match crate::voice::recorded_with(kind) {
                None => crate::voice::line_audio(kind, line),
                Some((voice, _)) => {
                    let rec = crate::voice::from_wav(&std::fs::read(format!("{}/{}.wav", raw, name)).expect("a recording")).expect("a wav");
                    crate::voice::process_from(&rec, kind, crate::voice::recorded_pitch(voice))
                }
            };
            std::fs::write(format!("{}/{}.wav", out, name), crate::voice::to_wav(&crate::voice::trim(&audio))).unwrap();
        }
    }
}

#[test]
fn every_lord_line_has_a_voice_key() {
    for kind in crate::world::ALL_LORDS {
        for (i, line) in kind.lines().iter().enumerate().filter(|(_, l)| !l.is_empty()) {
            assert_eq!(crate::voice::line_key(line), Some(format!("{}_{}", crate::voice::key(kind), i)), "{}", line);
            assert!(crate::embedded::voice(&format!("{}_{}", crate::voice::key(kind), i)).is_some(), "no recording of {:?}", line);
        }
    }
    assert_eq!(crate::voice::line_key("You are not him!").as_deref(), Some("bride_1"));
    assert_eq!(crate::voice::line_key("Nobody says this."), None);
}
