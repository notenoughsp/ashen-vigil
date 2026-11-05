//! Self-playing test mode with screenshots.
//!
//! ASHEN_AUTOTEST=<dir> runs a single-player script. With ASHEN_AUTOTEST_ROLE=host|client two
//! processes play together over localhost. ASHEN_WINDOW_POS=x,y moves the window, e.g. onto a
//! second monitor. Tests run muted and never touch the real save file.

use super::*;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Single,
    Host,
    Client,
    /// Worst case on screen: a huge late-game horde with every effect on.
    Stress,
    /// Only the menus, page after page for minutes, to find hitches and leaks there.
    Menus,
}

pub struct AutoTest {
    dir: String,
    pub role: Role,
    frame: u32,
    run_start: Option<u32>,
    choosing_since: Option<u32>,
    shots: Vec<&'static str>,
    /// Client only (ASHEN_AUTOTEST_REJOIN): leave mid-vigil and join again.
    rejoin: bool,
    /// ASHEN_AUTOTEST_NAME, so several clients can run side by side.
    name: Option<&'static str>,
    /// Host only (ASHEN_AUTOTEST_PLAYERS): how many players to wait for, host included.
    players: usize,
    /// ASHEN_AUTOTEST_AGAIN: everyone falls, the host goes back to the lobby and begins again.
    again: bool,
    /// Frames at which the client left and came back; host: a hero was seen gone.
    left_at: Option<u32>,
    /// Menus only: the frame a vigil started by a button began.
    in_vigil: Option<u32>,
}

fn screenshot(path: &str) {
    let mut img = get_screen_data();
    for px in img.bytes.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    img.export_png(path);
}

impl AutoTest {
    pub fn from_env() -> Option<AutoTest> {
        let dir = std::env::var("ASHEN_AUTOTEST").ok()?;
        let role = match std::env::var("ASHEN_AUTOTEST_ROLE").unwrap_or_default().as_str() {
            "host" => Role::Host,
            "client" => Role::Client,
            "stress" => Role::Stress,
            "menus" => Role::Menus,
            _ => Role::Single,
        };
        let rejoin = std::env::var("ASHEN_AUTOTEST_REJOIN").is_ok();
        let name = std::env::var("ASHEN_AUTOTEST_NAME").ok().map(|n| &*Box::leak(n.chars().take(16).collect::<String>().into_boxed_str()));
        let players = std::env::var("ASHEN_AUTOTEST_PLAYERS").ok().and_then(|v| v.parse().ok()).unwrap_or(2usize).clamp(2, crate::world::MAX_HEROES);
        let again = std::env::var("ASHEN_AUTOTEST_AGAIN").is_ok();
        Some(AutoTest { dir, role, frame: 0, run_start: None, choosing_since: None, shots: Vec::new(), rejoin, left_at: None, in_vigil: None, name, players, again })
    }

    pub fn log_file(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/{}log.txt", self.dir, self.prefix()))
    }

    pub fn player_name(&self) -> &'static str {
        if let Some(n) = self.name {
            return n;
        }
        match self.role {
            Role::Client => "Joiner",
            Role::Host => "Host",
            Role::Single | Role::Stress | Role::Menus => "Tester",
        }
    }

    /// Walks in loops and rolls now and then.
    pub fn input(&self) -> Input {
        let t = self.frame as f32 / 60.0 + if self.role == Role::Client { 2.0 } else { 0.0 };
        Input { dir: vec2(t.cos(), (t * 0.7).sin()).normalize_or_zero(), roll: self.frame.is_multiple_of(97), skill: self.frame.is_multiple_of(211) }
    }

    /// Picks a card after a while so the "waiting" screens can be captured.
    pub fn should_pick(&self) -> bool {
        let wait = if self.role == Role::Client { 150 } else { 40 };
        self.choosing_since.map(|s| self.frame > s + wait).unwrap_or(false)
    }

    fn prefix(&self) -> String {
        if let (Role::Client, Some(n)) = (self.role, self.name) {
            return format!("{}_", n.to_lowercase());
        }
        match self.role {
            Role::Single => "",
            Role::Host => "host_",
            Role::Client => "client_",
            Role::Stress => "stress_",
            Role::Menus => "menus_",
        }
        .to_string()
    }

    fn has(&self, name: &str) -> bool {
        self.shots.contains(&name)
    }
}

/// Gives a test hero a fused weapon, the new weapons and some items.
fn arm(w: &mut World, h: usize) {
    let hero = &mut w.heroes[h];
    hero.weapons.clear();
    for k in [WeaponKind::Blade, WeaponKind::Axe, WeaponKind::Scythe, WeaponKind::Hex, WeaponKind::Lance, WeaponKind::Crossbow] {
        hero.weapons.push(Weapon::new(k));
    }
    hero.weapons[0].level = WEAPON_MAX_LEVEL;
    hero.weapons[1].level = WEAPON_MAX_LEVEL;
    hero.apply_choice(Choice::NewItem(ItemKind::LeechFang));
    hero.apply_choice(Choice::NewItem(ItemKind::GraveCandle));
    hero.p.hp = 1.0e6;
    hero.stats.max_hp = 1.0e6;
    w.check_fusion(h);
}

impl App {
    pub fn autotest(&mut self) {
        let Some(au) = self.auto.as_mut() else { return };
        au.frame += 1;
        let (f, role) = (au.frame, au.role);
        let choosing = self.world.as_ref().map(|w| w.pause == Pause::Choosing && !w.heroes.is_empty()).unwrap_or(false);
        {
            let au = self.auto.as_mut().unwrap();
            if choosing {
                au.choosing_since.get_or_insert(f);
            } else {
                au.choosing_since = None;
            }
        }
        if f % 60 == 0 {
            let prefix = self.auto.as_ref().unwrap().prefix();
            let stats = self.net_stats().map(|s| format!(" ping {:.0}ms sent {:.1}KB/s recv {:.1}KB/s", s.ping_ms, s.sent_rate / 1024.0, s.recv_rate / 1024.0)).unwrap_or_default();
            match &self.world {
                Some(w) => println!(
                    "[{}] frame {} fps {} t {:.0} heroes {} enemies {} bullets {} decals {} snapshots {} pause {:?}{}",
                    prefix,
                    f,
                    get_fps(),
                    w.time,
                    w.heroes.len(),
                    w.enemies.len(),
                    w.ebullets.len(),
                    w.decals.len(),
                    self.snapshots,
                    w.pause,
                    stats
                ),
                None => println!("[{}] frame {} fps {} menus{}", prefix, f, get_fps(), stats),
            }
        }
        let want = match role {
            Role::Single => self.script_single(f),
            Role::Host => self.script_host(f),
            Role::Client => self.script_client(f),
            Role::Stress => self.script_stress(f),
            Role::Menus => self.script_menus(f),
        };
        let au = self.auto.as_mut().unwrap();
        for name in want {
            screenshot(&format!("{}/{}{}.png", au.dir, au.prefix(), name));
            au.shots.push(name);
            println!("[{}] screenshot {}", au.prefix(), name);
        }
    }

    fn script_single(&mut self, f: u32) -> Vec<&'static str> {
        let mut want = Vec::new();
        match f {
            4 => self.build_gallery(),
            14 => want.push("gallery"),
            15 => {
                self.world = None;
                self.save.settings.lighting = true;
                self.screen = Screen::Menu;
            }
            16 => self.screen = Screen::Credits,
            18 => want.push("credits"),
            19 => self.screen = Screen::Menu,
            20 => want.push("menu"),
            // The notice about a newer version, pretended: the self-test never asks GitHub.
            21 => self.updater = crate::update::Updater::pretend("9.9.9"),
            23 => want.push("menu_update"),
            24 => {
                self.updater = crate::update::Updater::off();
                self.screen = Screen::Reliquary;
            }
            30 => want.push("reliquary"),
            31 => {
                self.screen = Screen::Options;
                self.opt_page = 2;
            }
            40 => want.push("options"),
            41 => self.opt_page = 0,
            44 => want.push("controls"),
            45 => self.open_lobby(),
            100 => want.push("lobby_browse"),
            101 => {
                self.class_idx = self.classes().iter().position(|c| *c == Class::Reaper).unwrap_or(0);
                self.start_single();
                let w = self.world.as_mut().unwrap();
                arm(w, 0);
                w.time = 225.0;
                let p = w.heroes[0].p.pos;
                // A crowd right next to us so combat and gore show up at once.
                for i in 0..70 {
                    let kind = [EnemyKind::Hollow, EnemyKind::Ghoul, EnemyKind::Archer, EnemyKind::Bloat][i % 4];
                    let at = p + Vec2::from_angle(i as f32 * 0.37) * (180.0 + (i % 7) as f32 * 25.0);
                    let e = w.make_enemy(kind, at);
                    w.enemies.push(e);
                }
                if let Some(cat) = w.cat.as_mut() {
                    cat.pos = p + vec2(-90.0, 40.0);
                }
                for (i, kind) in [PickKind::Buff(Buff::Fury), PickKind::Relic, PickKind::Pyre, PickKind::Bread].into_iter().enumerate() {
                    let mut pk = Pickup::new(p + vec2(260.0 + i as f32 * 40.0, -120.0), kind);
                    pk.life = 60.0;
                    w.pickups.push(pk);
                }
            }
            300 => {
                // Visit a corner to look at the wall and the abyss.
                let w = self.world.as_mut().unwrap();
                w.heroes[0].p.pos = vec2(WORLD_HALF - 220.0, -WORLD_HALF + 260.0);
                w.cam = w.heroes[0].p.pos;
            }
            330 => want.push("wall"),
            331 => {
                let w = self.world.as_mut().unwrap();
                w.heroes[0].p.pos = Vec2::ZERO;
                w.cam = Vec2::ZERO;
                // Every map event at once, plus the late-game foes.
                w.blood_moon = 40.0;
                w.merchant = Some(Merchant {
                    pos: vec2(-260.0, -150.0),
                    life: 60.0,
                    wares: [Ware::Draught, Ware::Relic, Ware::Dice, Ware::Rage, Ware::Ward, Ware::Lodestone],
                    sold: [false, true, false, false, false, false],
                    prices: [40, 90, 150, 120, 100, 80],
                });
                w.cursed = Some(CursedShrine { pos: vec2(280.0, -170.0), life: 60.0, kindle: 0.0, curse: 0.0, accepted: false });
                for (i, kind) in [EnemyKind::Cultist, EnemyKind::Shieldbearer, EnemyKind::Necromancer].into_iter().enumerate() {
                    let e = w.make_enemy(kind, vec2(-120.0 + i as f32 * 120.0, 230.0));
                    w.enemies.push(e);
                }
                self.unlock(Achievement::MoonWatcher);
            }
            345 => want.push("events"),
            350 => self.save.settings.cute = true,
            380 => want.push("cute"),
            381 => self.save.settings.cute = false,
            400 => {
                // A quest in hand, for the HUD in the picture.
                let w = self.world.as_mut().unwrap();
                w.quest = Some(crate::world::Quest { kind: crate::world::QuestKind::Slay, goal: 150, progress: 37, left: 48.0 });
            }
            420 => want.push("run"),
            421 => self.screen = Screen::Paused,
            424 => want.push("pause"),
            425 => {
                self.screen = Screen::Options;
                self.options_in_game = true;
                self.opt_page = 1;
            }
            428 => want.push("options_ingame"),
            429 => {
                self.options_in_game = false;
                self.screen = Screen::Playing;
            }
            430 => {
                let w = self.world.as_mut().unwrap();
                w.time = w.next_boss - 4.0;
            }
            460 => want.push("warning"),
            900 => want.push("boss"),
            // The heroes' weapons faded, to see a lord's attacks through them.
            902 => self.save.settings.effect_alpha = 30,
            906 => want.push("boss_faded"),
            907 => self.save.settings.effect_alpha = 100,
            1000 => self.world.as_mut().unwrap().pending_levels += 1,
            1010 => want.push("levelup"),
            1300 => {
                let w = self.world.as_mut().unwrap();
                let h = &mut w.heroes[0];
                h.p.hp = 1.0;
                h.p.iframes = 0.0;
                h.p.roll_t = 0.0;
                h.buffs = [0.0; 3];
                w.hurt_hero(0, 999.0);
            }
            1520 => want.push("death"),
            1521 => {
                self.save.settings.boss_rush = true;
                self.save.challenges = Challenge::GlassCannon.bit() | Challenge::OneWeapon.bit();
                self.screen = Screen::Hub;
                self.hub_sel = 3;
            }
            1530 => want.push("hub"),
            1531 => {
                for (i, class) in ALL_CLASSES.iter().take(6).enumerate() {
                    let r = RunRecord {
                        time: 1800.0 - i as f32 * 211.0,
                        kills: 30000 - i as u32 * 3100,
                        level: 70 - i as u32 * 6,
                        players: 1 + i as u32 % 3,
                        won: i < 2,
                        class: *class,
                        difficulty: ["Abyss", "Cruel", "Normal"][i % 3].into(),
                        damage: 2.0e6 - i as f64 * 2.5e5,
                    };
                    self.save.record_run(r);
                }
                self.screen = Screen::Records;
                self.records_page = 0;
            }
            1540 => want.push("records"),
            1541 => self.records_page = 1,
            1550 => want.push("achievements"),
            1551 => self.records_page = 3,
            1555 => want.push("achievements3"),
            1556 => {
                self.screen = Screen::Wardrobe;
                self.save.souls = 60_000;
                self.save.skins = 0b1011_0101;
                self.save.profile.skin = 3;
                self.wardrobe_sel = 13;
            }
            1565 => want.push("wardrobe"),
            1566 => self.build_skin_gallery(false),
            1575 => want.push("skins"),
            1576 => self.build_skin_gallery(true),
            1585 => want.push("skins_cute"),
            1586 => {
                self.world = None;
                self.screen = Screen::Menu;
            }
            1595 => want.push("menu_cute"),
            1596 => {
                self.save.settings.cute = false;
                self.screen = Screen::Reliquary;
                self.relic_page = 1;
            }
            1605 => want.push("relics"),
            1606 => {
                // A boss rush: the first lord arrives within seconds.
                self.save.settings.boss_rush = true;
                self.save.challenges = 0;
                self.start_single();
            }
            1880 => {
                // Near the corner of the small rush field, so its walls are in the picture.
                if let Some(w) = self.world.as_mut() {
                    let at = vec2(crate::world::RUSH_HALF - 350.0, crate::world::RUSH_HALF - 250.0);
                    w.heroes[0].p.pos = at;
                }
            }
            1900 => want.push("rush"),
            1901 => self.field_showcase(Biome::Marsh, BossKind::Bride),
            2000 => want.push("marsh"),
            2001 => self.field_showcase(Biome::Cathedral, BossKind::Penitent),
            2100 => want.push("cathedral"),
            2101 => {
                // Drive the lord into its last phase to hear it.
                if let Some(e) = self.world.as_mut().and_then(|w| w.enemies.iter_mut().find(|e| e.boss)) {
                    e.hp = e.max_hp * 0.2;
                }
            }
            2140 => want.push("boss_phase"),
            2141 => {
                // An arsenal at work: six weapons, all well levelled, against a crowd.
                self.save.settings.biome = 0;
                self.class_idx = 0;
                self.start_single();
                let w = self.world.as_mut().unwrap();
                w.time = 6.0 * 60.0;
                w.next_boss = f32::MAX;
                let kinds = [WeaponKind::Lanterns, WeaponKind::Axe, WeaponKind::Orb, WeaponKind::Spear, WeaponKind::Daggers, WeaponKind::Hex];
                w.heroes[0].weapons = kinds.iter().map(|k| Weapon { level: 8, ..Weapon::new(*k) }).collect();
                w.heroes[0].recompute_stats();
                let p = w.heroes[0].p.pos;
                for i in 0..60 {
                    let kind = [EnemyKind::Hollow, EnemyKind::Ghoul, EnemyKind::Wraith, EnemyKind::Banshee][i % 4];
                    let e = w.make_enemy(kind, p + Vec2::from_angle(i as f32 * 0.41) * (160.0 + (i % 6) as f32 * 35.0));
                    w.enemies.push(e);
                }
            }
            2170 => want.push("arsenal"),
            2171 => {
                // The omens, all at once around the hero: an altar and a pact, the wager stone,
                // a cursed coffer, the courier and its vault, the mist's champion, the rival,
                // and the ash rain.
                let w = self.world.as_mut().unwrap();
                w.enemies.clear();
                w.projs.clear();
                w.time = 7.0 * 60.0;
                w.next_boss = w.time + 20.0;
                let p = w.heroes[0].p.pos;
                w.heroes[0].pact = true;
                w.heroes[0].p.hp = w.heroes[0].stats.max_hp * 0.6;
                w.altars.push(crate::world::Altar { pos: p + vec2(-330.0, -150.0), life: 50.0, kindle: 0.9 });
                w.stone = Some(crate::world::WagerStone { pos: p + vec2(300.0, -230.0) });
                w.pickups.push(Pickup::new(p + vec2(-170.0, 150.0), PickKind::CursedCoffer));
                w.pickups.push(Pickup::new(p + vec2(-60.0, 190.0), PickKind::Key));
                w.vault = Some(crate::world::Vault { pos: p + vec2(-520.0, 120.0), open: false });
                w.vault_key = true;
                w.weather = Some((crate::world::Weather::AshRain, 120.0));
                w.mist = 60.0;
                for (kind, at) in [(EnemyKind::Champion, vec2(260.0, 120.0)), (EnemyKind::Rival, vec2(110.0, -120.0)), (EnemyKind::Courier, vec2(-250.0, 0.0))] {
                    let mut e = w.make_enemy(kind, p + at);
                    if kind == EnemyKind::Champion {
                        e.elite = true;
                        e.r = kind.radius(true);
                    }
                    e.speed = 0.0;
                    e.hp = 1e9;
                    w.enemies.push(e);
                }
                w.heroes[0].wager = Some(crate::world::Wager { tier: 1, stake: 1234, since: None });
            }
            2180 => want.push("omens"),
            2181 => std::process::exit(0),
            _ => {}
        }
        want
    }

    /// Every class, enemy and boss side by side, frozen, unlit, for looking at the models.
    fn build_gallery(&mut self) {
        let mut w = World::new(Difficulty::default(), None);
        w.screen = crate::draw::view_size();
        w.cat = None;
        for (i, class) in ALL_CLASSES.iter().enumerate() {
            let slot = i as u8;
            let idx = w.add_hero(slot, class.name(), *class, Profile::default(), false);
            w.heroes[idx].p.pos = vec2(-440.0 + i as f32 * 110.0, 40.0);
            w.heroes[idx].p.facing = vec2(1.0, 0.0);
        }
        // The camera follows the local hero, so the middle one keeps everything in view.
        w.local = 4;
        let kinds = [
            EnemyKind::Hollow,
            EnemyKind::Ghoul,
            EnemyKind::Moth,
            EnemyKind::Leech,
            EnemyKind::Archer,
            EnemyKind::Bloat,
            EnemyKind::Wraith,
            EnemyKind::Banshee,
            EnemyKind::Cultist,
            EnemyKind::Gargoyle,
            EnemyKind::Shieldbearer,
            EnemyKind::Necromancer,
            EnemyKind::Knight,
        ];
        for (i, k) in kinds.iter().enumerate() {
            let mut e = w.make_enemy(*k, vec2(-600.0 + i as f32 * 100.0, -340.0));
            if *k == EnemyKind::Knight {
                e.elite = true;
                e.r *= 1.6;
            }
            w.enemies.push(e);
        }
        // All ten lords in two rows.
        for (i, k) in ALL_LORDS.iter().enumerate() {
            let at = vec2(-640.0 + (i % 5) as f32 * 320.0, if i < 5 { -170.0 } else { 260.0 });
            let mut e = w.make_enemy(EnemyKind::Boss(*k), at);
            e.boss = true;
            e.r = EnemyKind::Boss(*k).radius(false);
            w.enemies.push(e);
        }
        // Heroes face right; the enemies below them look up at the nearest one.
        w.cam = vec2(0.0, 40.0);
        // A finished world draws but no longer simulates.
        w.won = true;
        w.time = 12.3;
        self.save.settings.lighting = false;
        self.world = Some(w);
        self.screen = Screen::Playing;
    }

    /// A vigil on another field: its floor and props, its foes, braziers alight, a skill in
    /// use, and one of its lords.
    fn field_showcase(&mut self, biome: Biome, lord: BossKind) {
        self.save.settings.boss_rush = false;
        self.save.settings.biome = biome as u8;
        self.save.settings.lighting = true;
        self.save.challenges = 0;
        self.save.curses = 0;
        self.class_idx = self.classes().iter().position(|c| *c == Class::Arbalist).unwrap_or(0);
        self.start_single();
        let w = self.world.as_mut().unwrap();
        arm(w, 0);
        w.time = 9.0 * 60.0;
        w.next_boss = f32::MAX;
        let p = vec2(900.0, 700.0);
        w.heroes[0].p.pos = p;
        w.cam = p;
        // The braziers nearest the hero, alight.
        let mut near: Vec<usize> = (0..w.braziers.len()).collect();
        near.sort_by(|a, b| w.braziers[*a].pos.distance(p).total_cmp(&w.braziers[*b].pos.distance(p)));
        for i in near.into_iter().take(2) {
            w.braziers[i].burn = BRAZIER_BURN;
            w.heroes[0].p.pos = w.braziers[i].pos + vec2(150.0, 60.0);
        }
        let p = w.heroes[0].p.pos;
        w.cam = p;
        let kinds: &[EnemyKind] = match biome {
            Biome::Marsh => &[EnemyKind::Leech, EnemyKind::Ghoul, EnemyKind::Bloat, EnemyKind::Leech, EnemyKind::Wraith],
            _ => &[EnemyKind::Gargoyle, EnemyKind::Banshee, EnemyKind::Cultist, EnemyKind::Knight, EnemyKind::Shieldbearer],
        };
        for i in 0..40 {
            let at = p + Vec2::from_angle(i as f32 * 0.43) * (220.0 + (i % 5) as f32 * 40.0);
            let e = w.make_enemy(kinds[i % kinds.len()], at);
            w.enemies.push(e);
        }
        // The lord of the field, already fighting.
        w.bosses_spawned = biome.lords().iter().position(|k| *k == lord).unwrap_or(0) as u32;
        w.next_boss = w.time;
        w.heroes[0].skill_cd = 0.0;
        w.use_skill(0);
    }

    /// Every skin on a hero, frozen, plus a hero carrying the last flame for the HUD.
    fn build_skin_gallery(&mut self, cute: bool) {
        self.save.settings.cute = cute;
        let mut w = World::new(Difficulty::default(), None);
        w.screen = crate::draw::view_size();
        w.cat = None;
        let idx = w.add_hero(0, "Kitty", Class::Wanderer, Profile::default(), false);
        w.heroes[idx].p.pos = vec2(-470.0, 210.0);
        let mut last = Weapon::with_parts(WeaponKind::Blade, ALL_WEAPONS[1..].to_vec());
        last.whole = true;
        last.evolved = true;
        w.heroes[idx].weapons = vec![last];
        for (i, _) in SKINS.iter().enumerate() {
            let profile = Profile { skin: i as u8 + 1, ..Profile::default() };
            let class = ALL_CLASSES[i % ALL_CLASSES.len()];
            let h = w.add_hero(i as u8 + 1, "", class, profile, false);
            w.heroes[h].p.pos = vec2(-420.0 + (i % 9) as f32 * 105.0, -170.0 + (i / 9) as f32 * 150.0);
            w.heroes[h].p.facing = vec2(if i % 2 == 0 { 1.0 } else { -1.0 }, 0.0);
            w.heroes[h].aim = w.heroes[h].p.facing;
        }
        for i in 0..8 {
            let kind = [EnemyKind::Hollow, EnemyKind::Ghoul, EnemyKind::Moth, EnemyKind::Archer][i % 4];
            let e = w.make_enemy(kind, vec2(-300.0 + i as f32 * 90.0, 140.0));
            w.enemies.push(e);
        }
        w.local = 0;
        w.cam = vec2(0.0, -20.0);
        w.won = true;
        w.time = 12.3;
        w.cute = cute;
        // Flat light, so every skin shows its colours.
        self.save.settings.lighting = false;
        self.world = Some(w);
        self.screen = Screen::Playing;
    }

    fn script_menus(&mut self, f: u32) -> Vec<&'static str> {
        // Presses buttons like a player clicking around a new save: moves, confirms, changes
        // options, buys, pages, goes back. ASHEN_AUTOTEST_FRAMES long; ASHEN_AUTOTEST_PAGES=1
        // only flips through the screens instead.
        let end = std::env::var("ASHEN_AUTOTEST_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(18_000u32);
        if f >= end {
            std::process::exit(0);
        }
        crate::app::input::press_fake(None);
        // ASHEN_AUTOTEST_RESIZE=1: a player dragging the window bigger and smaller.
        if std::env::var("ASHEN_AUTOTEST_RESIZE").is_ok() && f % 20 == 10 {
            let k = (f / 20) % 40;
            request_new_screen_size(1500.0 + k as f32 * 9.0, 1090.0 + k as f32 * 7.0);
        }
        if std::env::var("ASHEN_AUTOTEST_PAGES").is_err() {
            // A vigil started by a button: let it run a little, then back to the menus.
            if matches!(self.screen, Screen::Playing | Screen::Paused | Screen::Dead { .. } | Screen::Victory { .. }) {
                let au = self.auto.as_mut().unwrap();
                let since = *au.in_vigil.get_or_insert(f);
                if f > since + 240 {
                    au.in_vigil = None;
                    self.world = None;
                    self.screen = Screen::Menu;
                }
                return Vec::new();
            }
            if !f.is_multiple_of(4) {
                return Vec::new();
            }
            // A small fixed random walk, the same every run.
            let r = (f.wrapping_mul(2_654_435_761) >> 7) % 100;
            let mut pad = match r {
                0..=34 => Pad::Down,
                35..=44 => Pad::Up,
                45..=69 => Pad::Confirm,
                70..=79 => Pad::Right,
                80..=84 => Pad::Left,
                85..=94 => Pad::Back,
                _ => Pad::PageNext,
            };
            let risky = match self.screen {
                // Exit would end the test.
                Screen::Menu => self.menu_sel == 7,
                Screen::Options => self.option_is_risky(),
                _ => false,
            };
            if risky && matches!(pad, Pad::Confirm | Pad::Left | Pad::Right) {
                pad = Pad::Down;
            }
            crate::app::input::press_fake(Some(pad));
            return Vec::new();
        }
        if f.is_multiple_of(15) {
            let step = (f / 15) % 16;
            match step {
                0 => self.screen = Screen::Menu,
                1 => self.screen = Screen::Hub,
                2 | 3 => {
                    self.screen = Screen::Reliquary;
                    self.relic_page = (step - 2) as usize;
                }
                4 => self.screen = Screen::Wardrobe,
                5..=8 => {
                    self.screen = Screen::Records;
                    self.records_page = (step - 5) as usize;
                }
                9..=13 => {
                    self.screen = Screen::Options;
                    self.opt_page = (step - 9) as usize;
                }
                14 => self.screen = Screen::Credits,
                _ => self.screen = Screen::Menu,
            }
        }
        Vec::new()
    }

    fn script_stress(&mut self, f: u32) -> Vec<&'static str> {
        let mut want = Vec::new();
        match f {
            5 => {
                let st = &mut self.save.settings;
                st.fps_cap = 0;
                // ASHEN_AUTOTEST_CUTE=1 measures the cute mode's hearts and flowers.
                st.cute = std::env::var("ASHEN_AUTOTEST_CUTE").is_ok_and(|v| v == "1");
                st.light_quality = 2;
                st.particles = 2;
                st.detail_high = true;
                st.diff = PRESETS[3].1;
                st.diff.count = 3.0;
                self.start_single();
                let w = self.world.as_mut().unwrap();
                arm(w, 0);
                w.time = 1450.0;
                let p = w.heroes[0].p.pos;
                for i in 0..1400 {
                    let kind = [EnemyKind::Hollow, EnemyKind::Ghoul, EnemyKind::Wraith, EnemyKind::Knight, EnemyKind::Archer, EnemyKind::Bloat, EnemyKind::Moth][i % 7];
                    let at = p + Vec2::from_angle(i as f32 * 0.618 * TAU) * (160.0 + (i % 40) as f32 * 22.0);
                    let e = w.make_enemy(kind, at);
                    w.enemies.push(e);
                }
            }
            300 => {
                // A floor full of single souls: the next kills must still drop orbs while the
                // old ones merge into bigger ones.
                let w = self.world.as_mut().unwrap();
                let me = w.heroes[0].p.pos;
                for i in 0..520 {
                    let at = me + Vec2::from_angle(i as f32 * 2.39996) * (120.0 + (i as f32).sqrt() * 22.0);
                    w.pickups.push(Pickup::new(clamp_world(at, 30.0), PickKind::Soul(1.0)));
                }
            }
            330 => {
                let w = self.world.as_ref().unwrap();
                let souls = w.pickups.iter().filter(|p| matches!(p.kind, PickKind::Soul(_))).count();
                let merged = w.pickups.iter().filter(|p| matches!(p.kind, PickKind::Soul(v) if v >= 2.0)).count();
                println!("[stress_] orbs on the floor: {} souls, {} grown by merging", souls, merged);
                want.push("orbs");
            }
            600 => want.push("horde"),
            4200 => std::process::exit(0),
            _ => {}
        }
        if let Some(w) = self.world.as_mut() {
            // Stay alive for the whole measurement.
            if let Some(h) = w.heroes.first_mut() {
                h.p.hp = h.stats.max_hp;
                h.p.iframes = 1.0;
            }
        }
        want
    }

    fn script_host(&mut self, f: u32) -> Vec<&'static str> {
        let mut want = Vec::new();
        if f == 10 {
            self.open_lobby();
            self.host_lobby();
            self.save.challenges = Challenge::GlassCannon.bit();
            self.save.settings.boss_rush = std::env::var("ASHEN_AUTOTEST_RUSH").is_ok_and(|v| v == "1");
        }
        let peers = match &self.net {
            NetMode::Host(h) => h.peers.len(),
            _ => 0,
        };
        let au = self.auto.as_ref().unwrap();
        if self.screen == Screen::Lobby && peers + 1 >= au.players && au.run_start.is_none() && f > 90 {
            want.push("lobby");
            self.auto.as_mut().unwrap().run_start = Some(f + 1);
        }
        let Some(s) = self.auto.as_ref().unwrap().run_start else {
            if f > 3000 {
                println!("[host_] no client joined, giving up");
                std::process::exit(1);
            }
            return want;
        };
        if f == s {
            self.start_hosted();
            let w = self.world.as_mut().unwrap();
            for h in 0..w.heroes.len() {
                arm(w, h);
            }
            w.time = 225.0;
        }
        if f == s + 240 {
            want.push("run");
            self.world.as_mut().unwrap().pending_levels += 1;
        }
        if f == s + 252 {
            want.push("levelup");
        }
        if f == s + 330 {
            want.push("waiting");
        }
        let countdown = self.world.as_ref().map(|w| matches!(w.pause, Pause::Countdown(_))).unwrap_or(false);
        if countdown && !self.auto.as_ref().unwrap().has("countdown") {
            want.push("countdown");
        }
        if f == s + 600 {
            let w = self.world.as_mut().unwrap();
            w.time = w.next_boss - 1.0;
        }
        if f == s + 1000 {
            want.push("boss");
        }
        // A client that leaves and comes back: shoot once everyone is connected again.
        if let Some(w) = &self.world {
            let au = self.auto.as_mut().unwrap();
            if w.heroes.iter().any(|h| !h.connected) {
                au.left_at.get_or_insert(f);
            } else if au.left_at.is_some_and(|l| f > l + 60) && !au.has("rejoined") {
                want.push("rejoined");
            }
        }
        if std::env::var("ASHEN_AUTOTEST_ENDLESS").is_ok() {
            match f.saturating_sub(s) {
                1100..=1250 => {
                    // Dawn, with no level-up or boss in the way.
                    let w = self.world.as_mut().unwrap();
                    w.next_boss = f32::MAX;
                    w.pending_levels = 0;
                    // A boss still fighting holds back the dawn, as in the real game.
                    w.enemies.retain(|e| !e.boss);
                    w.boss = None;
                    w.boss_kind = None;
                    w.arena = None;
                    if w.pause == Pause::Choosing {
                        for h in 0..w.heroes.len() {
                            let round = w.level_round;
                            w.pick(h, round, 0);
                        }
                    }
                    if w.pause != Pause::Choosing {
                        // A level-up countdown would hold the dawn back too.
                        w.pause = Pause::Running;
                    }
                    if w.boss_rush {
                        // A boss rush ends with its last lord, not at dawn.
                        w.bosses_killed = w.bosses_killed.max(RUSH_LORDS);
                    } else if w.time < VICTORY_TIME {
                        w.time = VICTORY_TIME + 0.5;
                        // Straight to dawn, without the Sovereign of the last hour.
                        w.finale_came = true;
                    }
                }
                1340 => {
                    println!("[host_] end screen: {:?}", self.screen);
                    want.push("victory");
                    self.continue_endless();
                }
                1500 => {
                    let w = self.world.as_ref().unwrap();
                    println!("[host_] endless {} won {} screen {:?} heroes {}", w.endless, w.won, self.screen, w.heroes.len());
                    want.push("endless");
                }
                1600 => std::process::exit(0),
                _ => {}
            }
            return want;
        }
        let again = self.auto.as_ref().unwrap().again;
        if again {
            match f.saturating_sub(s) {
                1100 => {
                    // Everyone falls at once.
                    let w = self.world.as_mut().unwrap();
                    for h in 0..w.heroes.len() {
                        let hero = &mut w.heroes[h];
                        hero.p.hp = 1.0;
                        hero.p.iframes = 0.0;
                        hero.p.roll_t = 0.0;
                        hero.buffs = [0.0; 3];
                        hero.p.revives = 0;
                    }
                    for h in 0..w.heroes.len() {
                        w.hurt_hero(h, 9999.0);
                    }
                }
                1300 => {
                    println!("[host_] end screen: {:?}", self.screen);
                    self.end_vigil();
                }
                1360 => want.push("lobby_again"),
                1600 => {
                    self.lobby.sel = 2;
                    self.start_hosted();
                    println!("[host_] second vigil with {} heroes", self.world.as_ref().map(|w| w.heroes.len()).unwrap_or(0));
                }
                1900 => want.push("second_run"),
                2000 => std::process::exit(0),
                _ => {}
            }
            return want;
        }
        if f == s + 1300 {
            std::process::exit(0);
        }
        want
    }

    fn script_client(&mut self, f: u32) -> Vec<&'static str> {
        let mut want = Vec::new();
        if f == 30 {
            // Each named client plays another class.
            let n = self.auto.as_ref().unwrap().player_name().bytes().map(|b| b as usize).sum::<usize>();
            self.class_idx = n % self.classes().len();
            self.open_lobby();
            let local = format!("127.0.0.1:{}", crate::net::game_port());
            self.lobby.ip = local.clone();
            self.join(&local);
        }
        let au = self.auto.as_ref().unwrap();
        if self.lobby.state == LobbyState::Joined && self.screen == Screen::Lobby && !au.has("lobby") {
            want.push("lobby");
        }
        // Leave mid-vigil, then join the same vigil again.
        let (rejoin, start, left) = {
            let au = self.auto.as_ref().unwrap();
            (au.rejoin, au.run_start, au.left_at)
        };
        if let (true, Some(s), None) = (rejoin, start, left) {
            if f == s + 400 {
                self.leave_online();
                self.world = None;
                self.screen = Screen::Menu;
                self.auto.as_mut().unwrap().left_at = Some(f);
                println!("[client_] left the vigil");
            }
        }
        if let (true, Some(l)) = (rejoin, left) {
            if f == l + 90 {
                self.open_lobby();
                let local = format!("127.0.0.1:{}", crate::net::game_port());
                self.lobby.ip = local.clone();
                self.join(&local);
                println!("[client_] joining again");
            }
            if f > l + 300 && self.world.as_ref().is_some_and(|w| !w.heroes.is_empty() && w.me().connected) && !self.auto.as_ref().unwrap().has("rejoined") {
                println!("[client_] back in the vigil at t {:.0}", self.world.as_ref().unwrap().time);
                want.push("rejoined");
            }
        }
        if let Some(w) = &self.world {
            if !w.heroes.is_empty() {
                let s = *self.auto.as_mut().unwrap().run_start.get_or_insert(f);
                let au = self.auto.as_ref().unwrap();
                if f == s + 230 {
                    want.push("run");
                }
                // A client looks at the options for a while: the vigil and the link go on.
                if f == s + 300 && self.screen == Screen::Playing {
                    self.screen = Screen::Options;
                    self.options_in_game = true;
                }
                if f == s + 420 && self.screen == Screen::Options {
                    let snaps = self.snapshots;
                    println!("[client_] options open for 2 s, {} snapshots so far", snaps);
                    want.push("options_online");
                    self.options_in_game = false;
                    self.screen = Screen::Playing;
                }
                if w.pause == Pause::Choosing && !w.me().picked && !au.has("levelup") {
                    want.push("levelup");
                }
                if w.boss_kind.is_some() && f.is_multiple_of(100) && !au.has("boss") {
                    want.push("boss");
                }
            }
        }
        if matches!(self.screen, Screen::Dead { .. }) && !self.auto.as_ref().unwrap().has("end") {
            want.push("end");
        }
        // Following the host into the endless night from the victory screen.
        if matches!(self.screen, Screen::Victory { .. }) && !self.auto.as_ref().unwrap().has("victory") {
            println!("[client_] on the victory screen");
            want.push("victory");
        }
        if let Some(w) = &self.world {
            let au = self.auto.as_ref().unwrap();
            if w.endless && !w.won && self.screen == Screen::Playing && !au.has("endless") {
                println!("[client_] fighting on in the endless night at {:.0}s", w.time);
                want.push("endless");
            }
        }
        // After the end screen: following the host into the lobby, then into the next vigil.
        let au = self.auto.as_ref().unwrap();
        if au.has("end") && self.screen == Screen::Lobby && !au.has("lobby_again") {
            println!("[client_] followed the host back to the lobby");
            want.push("lobby_again");
        }
        if au.has("lobby_again") && self.screen == Screen::Playing && self.world.as_ref().is_some_and(|w| !w.heroes.is_empty() && w.time > 2.0) && !au.has("second_run") {
            println!("[client_] in the second vigil");
            want.push("second_run");
        }
        if f > 3200 {
            std::process::exit(0);
        }
        want
    }
}
