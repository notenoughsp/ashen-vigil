//! Headless stress and robustness tests. Run with `cargo test --release -- --nocapture`.

use crate::data::*;
use crate::net::{self, Assembler, Reader};
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
            h.input = Input { dir: vec2(t.cos(), (t * 0.7).sin()).normalize_or_zero(), roll: f % (40 + i as u32) == 0 };
            h.auto_aim = i % 2 == 0;
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
    assert!(h.weapons.iter().any(|wp| wp.kind == WeaponKind::Blade && wp.fused == Some(WeaponKind::Axe)));
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
    let mut kinds = [false; 16];
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
    for k in [EnemyKind::Cultist, EnemyKind::Shieldbearer, EnemyKind::Necromancer] {
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
    simulate(&mut w, VICTORY_TIME + 200.0, true, |_, _| {});
    assert!(w.won, "the vigil should be kept at dawn");
    let bosses = w.bosses_killed;
    w.won = false;
    w.endless = true;
    w.next_boss = w.time + 60.0;
    let peaks = simulate(&mut w, 900.0, true, |_, _| {});
    assert_bounded(&peaks);
    assert!(!w.won, "an endless vigil never ends in a win");
    assert!(w.time > VICTORY_TIME + 900.0, "the endless vigil stopped at {}", w.time);
    assert!(w.bosses_killed > bosses, "no boss came after dawn");
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
