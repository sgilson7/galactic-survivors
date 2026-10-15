//! One run: the field, the players, the enemies and the schedule, stepped at
//! a fixed rate. Every rule of play lives here and in `arsenal.rs` (the
//! abilities) and `offer.rs` (level-up cards). The world takes each seat's
//! input for a tick and nothing else, so two browsers fed the same inputs
//! hold the same world, which is what co-op lockstep needs.

use crate::content::{Behaviour, Content};
use crate::geom::{dist2, from_angle, per_tick, px, ticks, within, TPS, V};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Most seats in a run.
pub const SEATS: usize = 2;

/// Size of a collision cell, in pixels.
const CELL_PX: i32 = 128;

/// Momentum counts at most this many seconds of moving (T14).
pub const MOMENTUM_SECONDS: i32 = 4;
/// Anchor needs this many ticks standing still.
pub const ANCHOR_TICKS: i32 = 30;

/// One seat's input for one tick. Directions are in -127..=127 on each axis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub mx: i8,
    pub my: i8,
    pub ax: i8,
    pub ay: i8,
    pub flags: u8,
    /// 0 none; 1..=3 take card k; 4..=6 banish card k-3; 7 reroll.
    pub card: u8,
    /// 0 none; otherwise a move in the rig window (`rig::Move::pack`).
    pub rig: u16,
}

pub const AIM: u8 = 1;
pub const DASH: u8 = 2;
pub const CARD_REROLL: u8 = 7;

impl Input {
    pub fn pack(self) -> u64 {
        (self.mx as u8 as u64)
            | (self.my as u8 as u64) << 8
            | (self.ax as u8 as u64) << 16
            | (self.ay as u8 as u64) << 24
            | (self.flags as u64) << 32
            | (self.card as u64) << 40
            | (self.rig as u64) << 48
    }

    pub fn unpack(v: u64) -> Input {
        Input {
            mx: (v & 0xff) as u8 as i8,
            my: ((v >> 8) & 0xff) as u8 as i8,
            ax: ((v >> 16) & 0xff) as u8 as i8,
            ay: ((v >> 24) & 0xff) as u8 as i8,
            flags: ((v >> 32) & 0xff) as u8,
            card: ((v >> 40) & 0xff) as u8,
            rig: ((v >> 48) & 0xffff) as u16,
        }
    }
}

/// The stats a player brings into a run, from the shop, modules and
/// achievements, plus the passives taken during it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub damage_pct: i32,
    pub cooldown_pct: i32,
    pub area_pct: i32,
    pub projectiles: i32,
    pub speed_pct: i32,
    pub max_hp: i32,
    pub armor: i32,
    pub pickup_pct: i32,
    pub rerolls: i32,
    pub banishes: i32,
    pub revives: i32,
    pub starter_levels: i32,
    pub shield_s: i32,
    pub rift_tickets: i32,
    pub xp_pct: i32,
    // passives added for rigs and runes (T14)
    #[serde(default)]
    pub momentum_pct: i32,
    #[serde(default)]
    pub anchor_pct: i32,
    #[serde(default)]
    pub rune_level: i32,
    #[serde(default)]
    pub heat_sink_pct: i32,
    #[serde(default)]
    pub double_tap: i32,
    #[serde(default)]
    pub fuse_area_pct: i32,
    #[serde(default)]
    pub sockets: i32,
    #[serde(default)]
    pub formation_extra: i32,
}

impl Stats {
    /// Adds `v` to the named stat. The names are `content::STATS`.
    pub fn add(&mut self, stat: &str, v: i32) {
        let field = match stat {
            "damage_pct" => &mut self.damage_pct,
            "cooldown_pct" => &mut self.cooldown_pct,
            "area_pct" => &mut self.area_pct,
            "projectiles" => &mut self.projectiles,
            "speed_pct" => &mut self.speed_pct,
            "max_hp" => &mut self.max_hp,
            "armor" => &mut self.armor,
            "pickup_pct" => &mut self.pickup_pct,
            "rerolls" => &mut self.rerolls,
            "banishes" => &mut self.banishes,
            "revives" => &mut self.revives,
            "starter_levels" => &mut self.starter_levels,
            "shield_s" => &mut self.shield_s,
            "rift_tickets" => &mut self.rift_tickets,
            "xp_pct" => &mut self.xp_pct,
            "momentum_pct" => &mut self.momentum_pct,
            "anchor_pct" => &mut self.anchor_pct,
            "rune_level" => &mut self.rune_level,
            "heat_sink_pct" => &mut self.heat_sink_pct,
            "double_tap" => &mut self.double_tap,
            "fuse_area_pct" => &mut self.fuse_area_pct,
            "sockets" => &mut self.sockets,
            "formation_extra" => &mut self.formation_extra,
            _ => return,
        };
        *field += v;
    }
}

/// What one player brings into a run. In co-op both browsers hold both
/// loadouts, so each simulates the other player exactly.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loadout {
    pub character: String,
    pub stats: Stats,
    pub autoaim: bool,
    /// Evolutions this player has not unlocked yet.
    pub locked: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// An ability or evolution id.
    pub id: String,
    pub level: i32,
    pub cd: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardKind {
    NewAbility,
    AbilityUp,
    NewPassive,
    PassiveUp,
    Gold,
    Evolve,
    Heal,
    Salvage,
    /// A part for a rig (T15).
    Part,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub kind: CardKind,
    /// The ability, passive or evolution the card names; empty for gold,
    /// heal and salvage.
    pub id: String,
    /// The level the card takes it to.
    pub level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferKind {
    LevelUp,
    Reward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub kind: OfferKind,
    pub cards: Vec<Card>,
}

/// What a player did in this run, for the results screen and achievements.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub kills: u32,
    pub kills_by: BTreeMap<String, u32>,
    /// Kills by enemy kind.
    pub kills_of: BTreeMap<String, u32>,
    /// The tick of the first hit taken; 0 while untouched.
    pub first_hit: i32,
    /// The tick of the first evolution; 0 for none.
    pub first_evolve: i32,
    pub dashes: u32,
    /// Times this player stood by a partner until they got back up.
    pub revives: u32,
    pub evolved: Vec<String>,
    pub gold: i32,
    pub tickets: i32,
    /// Runes drawn, by shape (T14).
    #[serde(default)]
    pub runes: BTreeMap<String, u32>,
    /// Enemies defeated by ground fields.
    #[serde(default)]
    pub field_kills: u32,
    /// Fields that met and reacted.
    #[serde(default)]
    pub reactions: u32,
    /// The deepest a trigger fired: 1 for a trigger on a rig's payload.
    #[serde(default)]
    pub deepest_trigger: u32,
    /// The most parts in one rig that fired.
    #[serde(default)]
    pub fullest_rig: u32,
}

/// One rig (T15): a row of sockets read left to right whenever it fires.
/// A socket holds an ability id, a part id, or "" for empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rig {
    pub sockets: Vec<String>,
    pub cd: i32,
    /// Times it has fired, for Double Tap.
    pub fired: u32,
    /// Distance walked toward the next stride trigger, and ticks toward the
    /// next clock trigger.
    pub stride: i32,
    pub clock: i32,
    /// Heat Sink's share of a tick, in hundredths.
    pub heat: i32,
}

/// A rune's stacking passive, with the ticks it has left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuneStack {
    pub shape: crate::rune::Shape,
    pub left: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub seat: usize,
    pub character: usize,
    pub load: Loadout,
    pub pos: V,
    pub facing: V,
    pub aim: V,
    pub aiming: bool,
    pub moving: bool,
    pub hp: i32,
    pub alive: bool,
    pub invulnerable: i32,
    pub hurt_flash: i32,
    pub dash_cd: i32,
    pub dash_left: i32,
    pub dash_dir: V,
    pub abilities: Vec<Slot>,
    pub passives: Vec<Slot>,
    pub rerolls: i32,
    pub banishes: i32,
    pub banished: Vec<String>,
    pub offer: Option<Offer>,
    pub pending: i32,
    pub revives: i32,
    pub revive_progress: i32,
    pub ghost_drop: i32,
    pub shield_cd: i32,
    pub record: Record,
    /// The last card action, so a held key takes one card.
    pub last_card: u8,
    /// Ticks of shield left while choosing a card in co-op (T13).
    #[serde(default)]
    pub guard: i32,
    /// The shield was given for the card open now, or not yet spent.
    #[serde(default)]
    pub guard_given: bool,
    /// The rigs and the spare parts (T15).
    #[serde(default)]
    pub rigs: Vec<Rig>,
    #[serde(default)]
    pub spare: Vec<String>,
    /// The walked trail, a point every few pixels, for runes (T14).
    #[serde(default)]
    pub trail: Vec<V>,
    #[serde(default)]
    pub rune_stacks: Vec<RuneStack>,
    /// Ticks spent moving without a stop, and standing still.
    #[serde(default)]
    pub moving_ticks: i32,
    #[serde(default)]
    pub still_ticks: i32,
    /// This tick's events, for player triggers: distance walked, a dash
    /// begun, and the centre of a rune drawn.
    #[serde(default)]
    pub walked: i32,
    #[serde(default)]
    pub dashed: bool,
    #[serde(default)]
    pub rune_at: Option<V>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Foe {
    pub id: u32,
    pub kind: usize,
    pub pos: V,
    pub push: V,
    pub hp: i32,
    pub max_hp: i32,
    pub freeze: i32,
    pub flash: i32,
    pub slow: i32,
    pub slow_pct: i32,
    pub contact: [i32; SEATS],
    pub fire_cd: i32,
    pub spawn_cd: i32,
    /// Charger: ticks into the current charge cycle; burst: the spiral's turn.
    pub phase: i32,
    pub dir: V,
    pub facing_left: bool,
}

/// A player's projectile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shot {
    pub seat: usize,
    pub ability: String,
    pub pos: V,
    pub vel: V,
    pub damage: i32,
    pub radius: i32,
    pub pierce: i32,
    pub bounce: i32,
    pub homing: bool,
    pub life: i32,
    pub hit: Vec<u32>,
    /// What a trigger after this payload fires, and when (T15).
    #[serde(default)]
    pub carry: Option<crate::rig::Carry>,
    /// The ground field it leaves where it hits, and the burst where it ends.
    #[serde(default)]
    pub field: Option<crate::field::FieldKind>,
    #[serde(default)]
    pub burst_pct: i32,
    /// A Boomerang Disc: turns for home halfway.
    #[serde(default)]
    pub returns: bool,
    #[serde(default)]
    pub full_life: i32,
    #[serde(default)]
    pub slow_pct: i32,
    #[serde(default)]
    pub slow: i32,
    #[serde(default)]
    pub pull: i32,
}

/// An enemy projectile. Every enemy shot has one shape on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bolt {
    pub pos: V,
    pub vel: V,
    pub damage: i32,
    pub radius: i32,
    pub life: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    /// A ring that grows from the player: a nova, a slam, an explosion.
    Ring,
    /// A wedge in front of the player: a sweep or a flame.
    Wedge,
    /// A straight line that keeps hurting for a while.
    Beam,
    /// Lines from enemy to enemy.
    Chain,
    /// A charge waiting to go off: a mine or a mortar shell.
    Bomb,
    /// Bodies circling the player.
    Orbit,
    /// A placed gun.
    Turret,
    /// A marked point struck from orbit when its fuse runs out.
    Strike,
    /// A pit pulling enemies in.
    Well,
    /// A line from the player to the partner or a placed effect.
    Tether,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub kind: EffectKind,
    pub seat: usize,
    pub ability: String,
    pub pos: V,
    pub dir: V,
    pub radius: i32,
    pub width: i32,
    pub damage: i32,
    pub life: i32,
    pub full: i32,
    pub every: i32,
    pub cd: i32,
    pub count: i32,
    pub angle: i32,
    pub spin: i32,
    pub armed: bool,
    pub points: Vec<V>,
    /// Foe id and the tick it may be hit again.
    pub hits: Vec<(u32, u32)>,
    pub shot_speed: i32,
    pub shot_radius: i32,
    pub shot_range: i32,
    /// Stays where it was made instead of following the player: anything
    /// a trigger or a formation placed away from the player (T15).
    #[serde(default)]
    pub anchored: bool,
    #[serde(default)]
    pub carry: Option<crate::rig::Carry>,
    #[serde(default)]
    pub field: Option<crate::field::FieldKind>,
    #[serde(default)]
    pub burst_pct: i32,
    #[serde(default)]
    pub pull: i32,
    #[serde(default)]
    pub slow_pct: i32,
    #[serde(default)]
    pub slow: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickupKind {
    Xp,
    Gold,
    Health,
    Cache,
    Ticket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pickup {
    pub kind: PickupKind,
    pub pos: V,
    pub value: i32,
    /// The seat it is flying to, once in a player's reach.
    pub to: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rift {
    pub pos: V,
    pub left: i32,
}

/// The field held aside while the players are inside a rift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stash {
    pub foes: Vec<Foe>,
    pub bolts: Vec<Bolt>,
    pub pickups: Vec<Pickup>,
    pub effects: Vec<Effect>,
    pub positions: Vec<V>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Playing,
    Won,
    Lost,
}

/// A message the screen shows for a while: the key of a string and how long.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Banner {
    pub key: String,
    pub left: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub tick: u32,
    pub rng: Rng,
    pub players: Vec<Player>,
    pub foes: Vec<Foe>,
    pub shots: Vec<Shot>,
    pub bolts: Vec<Bolt>,
    pub effects: Vec<Effect>,
    pub pickups: Vec<Pickup>,
    pub next_id: u32,
    pub xp: i32,
    pub level: i32,
    pub wave_cd: Vec<i32>,
    pub raid_at: i32,
    pub raid_cd: Vec<i32>,
    pub raid_boss_done: bool,
    pub boss_spawned: Vec<bool>,
    pub final_spawned: bool,
    pub rifts_done: usize,
    pub rift: Option<Rift>,
    pub in_rift: bool,
    pub rift_left: i32,
    pub stash: Option<Stash>,
    pub phase: Phase,
    pub banner: Option<Banner>,
    pub bosses_killed: Vec<String>,
    pub rifts_cleared: u32,
    pub raid_survived: bool,
    pub coop: bool,
    pub rift_cleared_now: bool,
    /// Ground fields (T14).
    #[serde(default)]
    pub fields: Vec<crate::field::Field>,
    /// Payloads waiting to fire again (Echo).
    #[serde(default)]
    pub pending: Vec<crate::rig::Pending>,
    /// While a payload fires: where it hurt a foe, and whether that foe fell
    /// (for a trigger after an instant payload). Not part of the world.
    #[serde(skip)]
    pub hit_log: Option<Vec<(V, bool)>>,
    /// Collision cells: foe indices per cell, rebuilt each tick.
    #[serde(skip)]
    pub cells: Vec<Vec<u16>>,
}

/// The world's tuning numbers converted once from the content file.
pub struct Tune {
    pub half: i32,
    pub cells_per_side: i32,
}

impl World {
    pub fn new(content: &Content, seed: u64, loadouts: &[Loadout]) -> World {
        let mut rng = Rng::new(seed);
        let run = &content.run;
        let coop = loadouts.len() > 1;
        let raid = &content.events.raid;
        let raid_at = ticks(1000) * rng.between(raid.from_s, raid.to_s.max(raid.from_s));
        let mut players = Vec::new();
        for (seat, load) in loadouts.iter().take(SEATS).enumerate() {
            let character = content
                .characters
                .iter()
                .position(|c| c.id == load.character)
                .unwrap_or(0);
            let ch = &content.characters[character];
            let start_level = (1 + load.stats.starter_levels).min(run.max_level);
            let max_hp = ch.hp + load.stats.max_hp;
            let offset = if !coop {
                0
            } else if seat == 0 {
                -40
            } else {
                40
            };
            players.push(Player {
                seat,
                character,
                load: load.clone(),
                pos: V::new(px(offset), 0),
                facing: V::new(px(1), 0),
                aim: V::new(px(1), 0),
                aiming: false,
                moving: false,
                hp: max_hp,
                alive: true,
                invulnerable: 0,
                hurt_flash: 0,
                dash_cd: 0,
                dash_left: 0,
                dash_dir: V::ZERO,
                abilities: vec![Slot {
                    id: ch.starter.clone(),
                    level: start_level,
                    cd: ticks(500),
                }],
                passives: Vec::new(),
                rerolls: run.rerolls + load.stats.rerolls,
                banishes: run.banishes + load.stats.banishes,
                banished: Vec::new(),
                offer: None,
                pending: 0,
                revives: load.stats.revives,
                revive_progress: 0,
                ghost_drop: 0,
                shield_cd: 0,
                record: Record::default(),
                last_card: 0,
                guard: 0,
                guard_given: false,
                rigs: Vec::new(),
                spare: Vec::new(),
                trail: Vec::new(),
                rune_stacks: Vec::new(),
                moving_ticks: 0,
                still_ticks: 0,
                walked: 0,
                dashed: false,
                rune_at: None,
            });
        }
        let mut w = World {
            tick: 0,
            rng,
            players,
            foes: Vec::new(),
            shots: Vec::new(),
            bolts: Vec::new(),
            effects: Vec::new(),
            pickups: Vec::new(),
            next_id: 1,
            xp: 0,
            level: 1,
            wave_cd: content.waves.iter().map(|_| 0).collect(),
            raid_at,
            raid_cd: raid.spawns.iter().map(|_| 0).collect(),
            raid_boss_done: false,
            boss_spawned: content.events.minibosses.iter().map(|_| false).collect(),
            final_spawned: false,
            rifts_done: 0,
            rift: None,
            in_rift: false,
            rift_left: 0,
            stash: None,
            phase: Phase::Playing,
            banner: None,
            bosses_killed: Vec::new(),
            rifts_cleared: 0,
            raid_survived: false,
            coop,
            rift_cleared_now: false,
            fields: Vec::new(),
            pending: Vec::new(),
            hit_log: None,
            cells: Vec::new(),
        };
        for p in w.players.iter_mut() {
            p.shield_cd = shield_ticks(content, &p.load.stats);
        }
        for seat in 0..w.players.len() {
            let starter = w.players[seat].abilities[0].id.clone();
            w.place_ability(content, seat, &starter);
        }
        w
    }

    pub fn tune(content: &Content) -> Tune {
        let half = px(content.run.map_half);
        Tune {
            half,
            cells_per_side: (content.run.map_half * 2) / CELL_PX + 2,
        }
    }

    pub fn seconds(&self) -> i32 {
        self.tick as i32 / TPS
    }

    /// Whether the field waits: a player holds an open card offer in a
    /// solo run. A co-op run goes on and shields the chooser instead (T13).
    pub fn waiting(&self) -> bool {
        !self.coop && self.players.iter().any(|p| p.offer.is_some())
    }

    pub fn xp_needed(content: &Content, level: i32) -> i32 {
        let r = &content.run;
        let mut need = r.xp_first + r.xp_step * (level - 1);
        if level > r.xp_late_level {
            need += r.xp_late_step * (level - r.xp_late_level);
        }
        need
    }

    // ---- stats ---------------------------------------------------------------

    pub fn stat_total(&self, content: &Content, seat: usize) -> Stats {
        let p = &self.players[seat];
        let mut s = p.load.stats.clone();
        for slot in &p.passives {
            if let Some(def) = content.passive(&slot.id) {
                s.add(&def.stat, def.per_level * slot.level);
            }
        }
        let ch = &content.characters[p.character];
        if ch.trait_ == "extra_shot" {
            s.projectiles += ch.trait_levels.iter().filter(|l| self.level >= **l).count() as i32;
        }
        // runes, Momentum and Anchor (T14)
        let r = &content.runes;
        for stack in &p.rune_stacks {
            match stack.shape {
                crate::rune::Shape::Circle => s.armor += r.circle_armor,
                crate::rune::Shape::Triangle => s.damage_pct += r.triangle_damage_pct,
                crate::rune::Shape::Square => s.area_pct += r.square_area_pct,
            }
        }
        if s.momentum_pct > 0 {
            s.damage_pct += s.momentum_pct * (p.moving_ticks / TPS).min(MOMENTUM_SECONDS);
        }
        if s.anchor_pct > 0 && p.still_ticks >= ANCHOR_TICKS {
            s.area_pct += s.anchor_pct;
        }
        s
    }

    pub fn max_hp(&self, content: &Content, seat: usize) -> i32 {
        let ch = &content.characters[self.players[seat].character];
        ch.hp + self.stat_total(content, seat).max_hp
    }

    fn move_speed(&self, content: &Content, seat: usize) -> i32 {
        let ch = &content.characters[self.players[seat].character];
        let pct = 100 + self.stat_total(content, seat).speed_pct;
        per_tick(ch.speed) * pct / 100
    }

    // ---- the tick ------------------------------------------------------------

    pub fn step(&mut self, content: &Content, inputs: &[Input]) {
        if self.phase != Phase::Playing {
            return;
        }
        self.read_cards(content, inputs);
        if self.waiting() {
            return;
        }
        self.tick += 1;
        if let Some(b) = self.banner.as_mut() {
            b.left -= 1;
            if b.left <= 0 {
                self.banner = None;
            }
        }
        self.move_players(content, inputs);
        if !self.in_rift {
            self.schedule(content);
        }
        self.rebuild_cells(content);
        self.move_foes(content);
        self.rebuild_cells(content);
        self.cast(content);
        self.move_shots(content);
        self.update_effects(content);
        self.update_fields(content);
        self.move_bolts(content);
        self.contact(content);
        self.collect(content);
        self.ghosts(content);
        if std::mem::take(&mut self.rift_cleared_now) {
            self.rift_won(content);
        }
        self.rift_tick(content);
        self.cull(content);
        self.check_end(content);
    }

    fn read_cards(&mut self, content: &Content, inputs: &[Input]) {
        // a move in the rig window, made on this tick (T15)
        for seat in 0..self.players.len() {
            let mv = inputs.get(seat).map(|i| i.rig).unwrap_or(0);
            if mv != 0 && self.players[seat].alive {
                self.rig_move(content, seat, mv);
            }
        }
        for seat in 0..self.players.len() {
            let card = inputs.get(seat).map(|i| i.card).unwrap_or(0);
            let fresh = card != self.players[seat].last_card;
            self.players[seat].last_card = card;
            if card == 0 || !fresh {
                continue;
            }
            self.card_action(content, seat, card);
        }
        for seat in 0..self.players.len() {
            if self.players[seat].offer.is_none() && self.players[seat].pending > 0 {
                self.players[seat].pending -= 1;
                let offer = self.level_offer(content, seat);
                self.players[seat].offer = Some(offer);
            }
        }
    }

    fn move_players(&mut self, content: &Content, inputs: &[Input]) {
        let half = px(content.run.map_half);
        for seat in 0..self.players.len() {
            let input = inputs.get(seat).copied().unwrap_or_default();
            let speed = self.move_speed(content, seat);
            let ch = &content.characters[self.players[seat].character];
            let (dash_ms, dash_speed, dash_cooldown) = (ch.dash_ms, ch.dash_speed, ch.dash_cooldown_ms);
            let is_dasher = ch.trait_ == "dash";
            let p = &mut self.players[seat];
            p.invulnerable = (p.invulnerable - 1).max(0);
            p.hurt_flash = (p.hurt_flash - 1).max(0);
            p.dash_cd = (p.dash_cd - 1).max(0);
            p.shield_cd = (p.shield_cd - 1).max(0);
            let dir = V::new(input.mx as i32, input.my as i32);
            p.moving = !dir.is_zero();
            p.dashed = false;
            if p.moving {
                p.moving_ticks += 1;
                p.still_ticks = 0;
            } else {
                p.moving_ticks = 0;
                p.still_ticks += 1;
            }
            // the chooser's shield: given when a card opens, kept while it is
            // open and afterwards until the player moves, for rearranging the
            // rigs, but never longer than choose_guard_ms in all
            if self.coop {
                if p.offer.is_some() {
                    if !p.guard_given {
                        p.guard = ticks(content.run.choose_guard_ms);
                        p.guard_given = true;
                    }
                } else if p.moving {
                    p.guard = 0;
                }
                p.guard = (p.guard - 1).max(0);
                if p.offer.is_none() && p.guard == 0 {
                    p.guard_given = false;
                }
            }
            if p.moving {
                p.facing = dir.with_len(px(1));
            }
            p.aiming = input.flags & AIM != 0 && !(input.ax == 0 && input.ay == 0);
            if p.aiming {
                p.aim = V::new(input.ax as i32, input.ay as i32).with_len(px(1));
            }
            if is_dasher && p.alive && input.flags & DASH != 0 && p.dash_cd == 0 {
                p.dash_left = ticks(dash_ms);
                p.dash_cd = ticks(dash_cooldown);
                p.dash_dir = p.facing;
                p.invulnerable = p.invulnerable.max(p.dash_left);
                p.record.dashes += 1;
                p.dashed = true;
            }
            let step = if p.dash_left > 0 {
                p.dash_left -= 1;
                p.dash_dir.with_len(per_tick(dash_speed))
            } else if !p.alive {
                dir.with_len(speed * 6 / 5)
            } else {
                dir.with_len(speed)
            };
            let was = p.pos;
            p.pos = p.pos.add(step).clamp_box(half);
            p.walked = (p.pos.sub(was).len() / crate::geom::SUB as i64) as i32;
        }
        for seat in 0..self.players.len() {
            if self.players[seat].alive {
                let r = content.run.player_radius;
                let pos = self.push_out(content, self.players[seat].pos, r);
                self.players[seat].pos = pos;
            }
        }
        if self.in_rift {
            let r = px(content.rift.arena_radius - content.run.player_radius);
            for p in self.players.iter_mut() {
                if p.pos.len() > r as i64 {
                    p.pos = p.pos.with_len(r);
                }
            }
        }
        for seat in 0..self.players.len() {
            self.wear_runes(seat);
            self.trace(content, seat);
        }
    }

    /// Moves a circle out of any obstacle it overlaps. Obstacles stay out
    /// of the rift arena.
    pub fn push_out(&self, content: &Content, pos: V, radius: i32) -> V {
        if self.in_rift {
            return pos;
        }
        let mut pos = pos;
        for o in &content.run.obstacles {
            let c = V::new(px(o.x), px(o.y));
            let min = px(o.r + radius);
            let d = pos.sub(c);
            if d.len2() < min as i64 * min as i64 {
                let away = if d.is_zero() { V::new(1, 0) } else { d };
                pos = c.add(away.with_len(min));
            }
        }
        pos
    }

    // ---- schedule ------------------------------------------------------------

    fn alive_seats(&self) -> Vec<usize> {
        self.players.iter().filter(|p| p.alive).map(|p| p.seat).collect()
    }

    fn anchor(&mut self) -> V {
        let seats = self.alive_seats();
        if seats.is_empty() {
            return self.players[0].pos;
        }
        let k = self.rng.below(seats.len() as u32) as usize;
        self.players[seats[k]].pos
    }

    fn spawn_point(&mut self, content: &Content, radius: i32) -> V {
        let anchor = self.anchor();
        let angle = self.rng.below(geom_turn()) as i32;
        let p = anchor.add(from_angle(angle, px(content.run.spawn_distance)));
        let p = p.clamp_box(px(content.run.map_half));
        self.push_out(content, p, radius)
    }

    pub fn spawn_foe(&mut self, content: &Content, kind: usize, pos: V) -> usize {
        let e = &content.enemies[kind];
        let minute = self.seconds() / 60;
        // a boss's health is set for the minute it arrives; everything
        // else grows tougher as the run goes on
        let scale = if e.boss {
            0
        } else {
            content.run.hp_scale_per_minute_pct * minute
        };
        let mut hp = e.hp * (100 + scale) / 100;
        if self.coop {
            hp = hp * content.run.coop_hp_pct / 100;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.foes.push(Foe {
            id,
            kind,
            pos,
            push: V::ZERO,
            hp,
            max_hp: hp,
            freeze: 0,
            flash: 0,
            slow: 0,
            slow_pct: 0,
            contact: [0; SEATS],
            fire_cd: ticks(e.fire_ms.max(e.charge_ms)),
            spawn_cd: ticks(e.spawn_ms),
            phase: 0,
            dir: V::ZERO,
            facing_left: false,
        });
        self.foes.len() - 1
    }

    fn spawn_group(&mut self, content: &Content, enemy: &str, count: i32) {
        let kind = content.enemy_index(enemy);
        let mut count = count;
        if self.coop {
            count = (count * content.run.coop_spawn_pct + 99) / 100;
        }
        for _ in 0..count {
            if self.foes.len() >= content.run.enemy_cap {
                return;
            }
            let pos = self.spawn_point(content, content.enemies[kind].radius);
            self.spawn_foe(content, kind, pos);
        }
    }

    fn say(&mut self, key: &str, ms: i32) {
        self.banner = Some(Banner {
            key: key.to_string(),
            left: ticks(ms),
        });
    }

    fn schedule(&mut self, content: &Content) {
        let t = self.tick as i32;
        let s = |sec: i32| sec * TPS;
        let final_on = self.final_spawned;
        for (k, w) in content.waves.iter().enumerate() {
            if final_on || t < s(w.from_s) || t >= s(w.to_s) {
                continue;
            }
            self.wave_cd[k] -= 1;
            if self.wave_cd[k] <= 0 {
                self.wave_cd[k] = ticks(w.every_ms);
                self.spawn_group(content, &w.enemy, w.count);
            }
        }
        for (k, b) in content.events.minibosses.iter().enumerate() {
            if !self.boss_spawned[k] && t >= s(b.at_s) {
                self.boss_spawned[k] = true;
                let kind = content.enemy_index(&b.enemy);
                let pos = self.spawn_point(content, content.enemies[kind].radius);
                self.spawn_foe(content, kind, pos);
                self.say("banner.boss", 3000);
            }
        }
        let raid = &content.events.raid;
        let warn = self.raid_at - ticks(raid.warning_ms);
        let end = self.raid_at + ticks(raid.length_ms);
        if t == warn {
            self.say("banner.raid_warning", raid.warning_ms);
        }
        if t >= self.raid_at && t < end {
            for (k, sp) in raid.spawns.iter().enumerate() {
                self.raid_cd[k] -= 1;
                if self.raid_cd[k] <= 0 {
                    self.raid_cd[k] = ticks(sp.every_ms);
                    self.spawn_group(content, &sp.enemy, sp.count);
                }
            }
        }
        if t == end && !self.raid_boss_done {
            self.raid_boss_done = true;
            let kind = content.enemy_index(&raid.boss);
            let pos = self.spawn_point(content, content.enemies[kind].radius);
            self.spawn_foe(content, kind, pos);
            self.say("banner.raid_boss", 3000);
        }
        if let Some(&at) = content.events.rifts_s.get(self.rifts_done) {
            if t >= s(at) && self.rift.is_none() {
                self.rifts_done += 1;
                let anchor = self.anchor();
                let angle = self.rng.below(geom_turn()) as i32;
                let pos = anchor.add(from_angle(angle, px(content.events.rift_distance)));
                let pos = pos.clamp_box(px(content.run.map_half - 100));
                let pos = self.push_out(content, pos, content.events.rift_radius + 20);
                self.rift = Some(Rift {
                    pos,
                    left: ticks(content.events.rift_open_ms),
                });
                self.say("banner.rift_open", 3000);
            }
        }
        if !self.final_spawned && t >= s(content.events.final_boss.at_s) {
            self.final_spawned = true;
            let kind = content.enemy_index(&content.events.final_boss.enemy);
            let pos = self.spawn_point(content, content.enemies[kind].radius);
            self.spawn_foe(content, kind, pos);
            self.say("banner.final", 4000);
        }
    }

    // ---- enemies -------------------------------------------------------------

    fn rebuild_cells(&mut self, content: &Content) {
        let side = World::tune(content).cells_per_side;
        let n = (side * side) as usize;
        if self.cells.len() != n {
            self.cells = vec![Vec::new(); n];
        }
        for c in self.cells.iter_mut() {
            c.clear();
        }
        for (i, f) in self.foes.iter().enumerate() {
            let k = self.cell_of(content, f.pos);
            self.cells[k].push(i as u16);
        }
    }

    fn cell_xy(&self, content: &Content, p: V) -> (i32, i32) {
        let side = World::tune(content).cells_per_side;
        let half = px(content.run.map_half);
        let cx = ((p.x + half) / px(CELL_PX)).clamp(0, side - 1);
        let cy = ((p.y + half) / px(CELL_PX)).clamp(0, side - 1);
        (cx, cy)
    }

    fn cell_of(&self, content: &Content, p: V) -> usize {
        let side = World::tune(content).cells_per_side;
        let (cx, cy) = self.cell_xy(content, p);
        (cy * side + cx) as usize
    }

    /// Indices of foes whose cells overlap the box around `p` of `r` pixels.
    pub fn near(&self, content: &Content, p: V, r: i32) -> Vec<usize> {
        let side = World::tune(content).cells_per_side;
        if self.cells.len() != (side * side) as usize {
            return (0..self.foes.len()).collect();
        }
        let reach = r / CELL_PX + 1;
        let (cx, cy) = self.cell_xy(content, p);
        let mut out = Vec::new();
        for y in (cy - reach).max(0)..=(cy + reach).min(side - 1) {
            for x in (cx - reach).max(0)..=(cx + reach).min(side - 1) {
                for &i in &self.cells[(y * side + x) as usize] {
                    if (i as usize) < self.foes.len() {
                        out.push(i as usize);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// Calls `f` with each foe whose cell overlaps the box around `p` of
    /// `r` pixels, in no set order and without a list: for sums that do
    /// not depend on order (crowd separation, which is most of a late tick).
    fn each_near(&self, content: &Content, p: V, r: i32, mut f: impl FnMut(usize)) {
        let side = World::tune(content).cells_per_side;
        if self.cells.len() != (side * side) as usize {
            (0..self.foes.len()).for_each(f);
            return;
        }
        let reach = r / CELL_PX + 1;
        let (cx, cy) = self.cell_xy(content, p);
        for y in (cy - reach).max(0)..=(cy + reach).min(side - 1) {
            for x in (cx - reach).max(0)..=(cx + reach).min(side - 1) {
                for &i in &self.cells[(y * side + x) as usize] {
                    if (i as usize) < self.foes.len() {
                        f(i as usize);
                    }
                }
            }
        }
    }

    fn nearest_player(&self, p: V) -> Option<usize> {
        self.players
            .iter()
            .filter(|q| q.alive)
            .min_by_key(|q| (dist2(p, q.pos), q.seat))
            .map(|q| q.seat)
    }

    fn move_foes(&mut self, content: &Content) {
        let n = self.foes.len();
        for i in 0..n {
            let kind = self.foes[i].kind;
            let e = &content.enemies[kind];
            let f = &mut self.foes[i];
            f.flash = (f.flash - 1).max(0);
            for c in f.contact.iter_mut() {
                *c = (*c - 1).max(0);
            }
            f.push = f.push.scale(7, 8);
            let pushed = f.pos.add(f.push);
            f.pos = pushed;
            if f.slow > 0 {
                f.slow -= 1;
            }
            if f.freeze > 0 {
                f.freeze -= 1;
                continue;
            }
            let Some(target) = self.nearest_player(self.foes[i].pos) else {
                continue;
            };
            let tp = self.players[target].pos;
            let f = &self.foes[i];
            let to = tp.sub(f.pos);
            let mut speed = per_tick(e.speed);
            if f.slow > 0 {
                speed = speed * (100 - f.slow_pct) / 100;
            }
            let dist = to.len();
            let mut step = to.with_len(speed);
            match e.behaviour {
                Behaviour::Chase | Behaviour::Burst => {}
                Behaviour::Ranged => {
                    let keep = px(e.keep) as i64;
                    if dist < keep - px(60) as i64 {
                        step = V::ZERO.sub(step);
                    } else if dist <= keep {
                        step = V::new(-to.y, to.x).with_len(speed / 2);
                    }
                }
                Behaviour::Charger => {
                    let cycle = ticks(e.charge_ms);
                    let wind = ticks(500);
                    let dash = ticks(600);
                    let phase = (f.phase + 1) % cycle.max(1);
                    let dir = f.dir;
                    let f = &mut self.foes[i];
                    f.phase = phase;
                    if phase == cycle - wind - dash {
                        f.dir = to.with_len(px(1));
                    }
                    if phase >= cycle - wind - dash && phase < cycle - dash {
                        step = V::ZERO;
                        f.flash = f.flash.max(2);
                    } else if phase >= cycle - dash {
                        step = dir.with_len(per_tick(e.charge_speed));
                    }
                }
            }
            let f = &mut self.foes[i];
            if step.x != 0 {
                f.facing_left = step.x < 0;
            }
            f.pos = f.pos.add(step);
        }
        self.separate(content);
        for i in 0..self.foes.len() {
            let r = content.enemies[self.foes[i].kind].radius;
            let pos = self.push_out(content, self.foes[i].pos, r);
            self.foes[i].pos = pos.clamp_box(px(content.run.map_half));
            if self.in_rift {
                let lim = px(content.rift.arena_radius - r);
                if self.foes[i].pos.len() > lim as i64 {
                    self.foes[i].pos = self.foes[i].pos.with_len(lim);
                }
            }
        }
        self.foe_attacks(content);
    }

    /// Overlapping foes push each other apart, so a crowd spreads into a
    /// readable blob instead of one stacked sprite.
    fn separate(&mut self, content: &Content) {
        let n = self.foes.len();
        let mut moves = vec![V::ZERO; n];
        for (i, mv) in moves.iter_mut().enumerate() {
            let a = &self.foes[i];
            let ra = content.enemies[a.kind].radius;
            // a sum of pushes: the order the neighbours come in is no matter
            self.each_near(content, a.pos, ra * 2, |j| {
                if j == i {
                    return;
                }
                let b = &self.foes[j];
                let rb = content.enemies[b.kind].radius;
                let min = px(ra + rb) * 4 / 5;
                let d = a.pos.sub(b.pos);
                let d2 = d.len2();
                if d2 < min as i64 * min as i64 {
                    let away = if d.is_zero() {
                        V::new(if i < j { 1 } else { -1 }, 0)
                    } else {
                        d
                    };
                    let overlap = min - d.len() as i32;
                    // the heavier foe moves less
                    let share = if rb >= ra { 1 } else { 3 };
                    *mv = mv.add(away.with_len((overlap / 4 * share / 2).max(1)));
                }
            });
        }
        for (f, mv) in self.foes.iter_mut().zip(moves) {
            f.pos = f.pos.add(mv.with_len((mv.len() as i32).min(px(3))));
        }
    }

    fn foe_attacks(&mut self, content: &Content) {
        let n = self.foes.len();
        for i in 0..n {
            let e = &content.enemies[self.foes[i].kind];
            let is_shooter = matches!(e.behaviour, Behaviour::Ranged | Behaviour::Burst) && e.fire_ms > 0;
            if self.foes[i].freeze > 0 {
                continue;
            }
            if !e.spawn.is_empty() {
                self.foes[i].spawn_cd -= 1;
                if self.foes[i].spawn_cd <= 0 {
                    self.foes[i].spawn_cd = ticks(e.spawn_ms);
                    let kind = content.enemy_index(&e.spawn);
                    let at = self.foes[i].pos;
                    for k in 0..e.spawn_count {
                        if self.foes.len() >= content.run.enemy_cap {
                            break;
                        }
                        let off =
                            from_angle(geom_turn() as i32 * k / e.spawn_count.max(1), px(e.radius + 30));
                        self.spawn_foe(content, kind, at.add(off));
                    }
                }
            }
            if !is_shooter {
                continue;
            }
            self.foes[i].fire_cd -= 1;
            if self.foes[i].fire_cd > 0 {
                continue;
            }
            self.foes[i].fire_cd = ticks(e.fire_ms);
            let from = self.foes[i].pos;
            let speed = per_tick(e.shot_speed);
            let radius = 8;
            let life = ticks(6000);
            if e.behaviour == Behaviour::Burst {
                let count = e.burst.max(1);
                let turn = geom_turn() as i32;
                self.foes[i].phase += turn / count / 2;
                let base = self.foes[i].phase;
                for k in 0..count {
                    let vel = from_angle(base + turn * k / count, speed);
                    self.add_bolt(content, from, vel, e.shot_damage, radius, life);
                }
            } else if let Some(t) = self.nearest_player(from) {
                let to = self.players[t].pos.sub(from);
                let volley = e.volley.max(1);
                for k in 0..volley {
                    let spread = (k - volley / 2) * 2400;
                    let vel = to.rotate(spread).with_len(speed);
                    self.add_bolt(content, from, vel, e.shot_damage, radius, life);
                }
            }
        }
    }

    fn add_bolt(&mut self, content: &Content, pos: V, vel: V, damage: i32, radius: i32, life: i32) {
        if self.bolts.len() >= content.run.shot_cap {
            return;
        }
        self.bolts.push(Bolt {
            pos,
            vel,
            damage,
            radius,
            life,
        });
    }

    fn move_bolts(&mut self, content: &Content) {
        for b in self.bolts.iter_mut() {
            b.pos = b.pos.add(b.vel);
            b.life -= 1;
        }
        let half = px(content.run.map_half);
        self.bolts
            .retain(|b| b.life > 0 && b.pos.x.abs() <= half && b.pos.y.abs() <= half);
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for (bi, b) in self.bolts.iter().enumerate() {
            for p in self.players.iter().filter(|p| p.alive) {
                if within(b.pos, p.pos, px(b.radius + content.run.player_radius)) {
                    hits.push((bi, p.seat));
                    break;
                }
            }
        }
        let mut gone = Vec::new();
        for (bi, seat) in hits {
            let dmg = self.scaled_damage(content, self.bolts[bi].damage);
            self.hurt_player(content, seat, dmg);
            gone.push(bi);
        }
        let mut k = 0;
        self.bolts.retain(|_| {
            let keep = !gone.contains(&k);
            k += 1;
            keep
        });
    }

    /// Enemies touching a player. Each enemy can hurt a given player at most
    /// once per contact window, and any hit makes the player briefly
    /// invulnerable, so a crowd cannot land many hits at once (I6).
    fn contact(&mut self, content: &Content) {
        let window = ticks(content.run.contact_window_ms);
        for seat in 0..self.players.len() {
            if !self.players[seat].alive {
                continue;
            }
            let ch = &content.characters[self.players[seat].character];
            let taken_pct = if ch.contact_taken_pct > 0 {
                ch.contact_taken_pct
            } else {
                100
            };
            let reflect = ch.reflect;
            let pos = self.players[seat].pos;
            let pr = content.run.player_radius;
            for i in self.near(content, pos, 120) {
                let e = &content.enemies[self.foes[i].kind];
                if !within(self.foes[i].pos, pos, px(e.radius + pr)) {
                    continue;
                }
                // invulnerability is checked once, in hurt_player, for every source
                if self.foes[i].contact[seat] > 0 {
                    continue;
                }
                self.foes[i].contact[seat] = window;
                let dmg = (self.scaled_damage(content, e.damage) * taken_pct / 100).max(1);
                self.hurt_player(content, seat, dmg);
                if reflect > 0 {
                    let ability = String::new();
                    self.hurt_foe(content, i, reflect, seat, &ability, V::ZERO);
                }
            }
        }
    }

    /// Enemy damage grows with the minute, as enemy health does.
    pub fn scaled_damage(&self, content: &Content, base: i32) -> i32 {
        base * (100 + content.run.damage_scale_per_minute_pct * (self.seconds() / 60)) / 100
    }

    pub fn hurt_player(&mut self, content: &Content, seat: usize, damage: i32) {
        let armor =
            self.stat_total(content, seat).armor + content.characters[self.players[seat].character].armor;
        let shield_full = shield_ticks(content, &self.players[seat].load.stats);
        let invulnerable = ticks(content.run.invulnerable_ms);
        let p = &mut self.players[seat];
        if !p.alive || p.invulnerable > 0 || p.guard > 0 {
            return;
        }
        if shield_full > 0 && p.shield_cd == 0 {
            p.shield_cd = shield_full;
            p.invulnerable = invulnerable;
            return;
        }
        let dmg = (damage - armor).max(1);
        p.hp -= dmg;
        if p.record.first_hit == 0 {
            p.record.first_hit = (self.tick as i32).max(1);
        }
        p.invulnerable = invulnerable;
        p.hurt_flash = ticks(200);
        if p.hp <= 0 {
            if p.revives > 0 {
                p.revives -= 1;
                p.hp = 1;
                p.invulnerable = ticks(2000);
                let max = self.max_hp(content, seat);
                self.players[seat].hp = max / 2;
                self.say("banner.revived", 2000);
            } else {
                p.hp = 0;
                p.alive = false;
                p.ghost_drop = ticks(content.run.ghost_drop_ms);
                p.revive_progress = 0;
            }
        }
    }

    /// Damages a foe; returns whether it died.
    pub fn hurt_foe(
        &mut self,
        content: &Content,
        i: usize,
        damage: i32,
        seat: usize,
        ability: &str,
        push: V,
    ) -> bool {
        if i >= self.foes.len() || self.foes[i].hp <= 0 {
            return false;
        }
        let e = &content.enemies[self.foes[i].kind];
        let resist = e.knock_resist_pct;
        let f = &mut self.foes[i];
        f.hp -= damage;
        f.flash = ticks(content.run.flash_ms);
        if !(e.boss || e.elite) {
            f.freeze = f.freeze.max(ticks(content.run.hit_freeze_ms));
        }
        if !push.is_zero() {
            f.push = f.push.add(push.scale(100 - resist, 100));
        }
        let alive = f.hp > 0;
        let at = f.pos;
        if let Some(log) = self.hit_log.as_mut() {
            log.push((at, !alive));
        }
        if alive {
            return false;
        }
        self.kill(content, i, seat, ability);
        true
    }

    fn kill(&mut self, content: &Content, i: usize, seat: usize, ability: &str) {
        let f = self.foes[i].clone();
        let e = &content.enemies[f.kind];
        let who = seat.min(self.players.len() - 1);
        let rec = &mut self.players[who].record;
        rec.kills += 1;
        if !ability.is_empty() {
            *rec.kills_by.entry(ability.to_string()).or_insert(0) += 1;
        }
        *rec.kills_of.entry(e.id.clone()).or_insert(0) += 1;
        if e.xp > 0 {
            self.drop(content, PickupKind::Xp, f.pos, e.xp);
        }
        if e.boss || e.elite {
            self.drop(content, PickupKind::Cache, f.pos.add(V::new(px(20), 0)), 1);
            self.drop(content, PickupKind::Health, f.pos.sub(V::new(px(20), 0)), 1);
        }
        if e.boss {
            self.drop(content, PickupKind::Ticket, f.pos.add(V::new(0, px(24))), 1);
            self.bosses_killed.push(e.id.clone());
            if e.id == content.events.raid.boss {
                self.raid_survived = true;
                self.say("banner.raid_over", 3000);
            }
            if e.is_final {
                self.phase = Phase::Won;
            }
        } else if self.rng.chance(content.run.gold_drop_pct, 100) {
            self.drop(
                content,
                PickupKind::Gold,
                f.pos.add(V::new(px(8), 0)),
                content.run.gold_chip,
            );
        } else if self.rng.chance(content.run.health_drop_pct, 100) {
            self.drop(content, PickupKind::Health, f.pos, 1);
        }
        if e.elite && self.in_rift {
            // finished after the tick: the rift swaps the field back, and
            // this kill may come from inside a loop over that field
            self.rift_cleared_now = true;
        }
        self.foes[i].hp = 0;
    }

    pub fn drop(&mut self, content: &Content, kind: PickupKind, pos: V, value: i32) {
        if self.pickups.len() >= content.run.pickup_cap && kind == PickupKind::Xp {
            // fold into the nearest shard so experience is never lost
            if let Some(k) = self
                .pickups
                .iter()
                .enumerate()
                .filter(|(_, p)| p.kind == PickupKind::Xp)
                .min_by_key(|(_, p)| dist2(p.pos, pos))
                .map(|(k, _)| k)
            {
                self.pickups[k].value += value;
            }
            return;
        }
        self.pickups.push(Pickup {
            kind,
            pos,
            value,
            to: None,
        });
    }

    // ---- pickups, levels -----------------------------------------------------

    fn collect(&mut self, content: &Content) {
        let base = content.run.pickup_radius;
        let reach: Vec<(usize, V, i32)> = self
            .players
            .iter()
            .filter(|p| p.alive)
            .map(|p| {
                let pct = 100 + self.stat_total(content, p.seat).pickup_pct;
                (p.seat, p.pos, px(base * pct / 100))
            })
            .collect();
        let speed = per_tick(content.run.magnet_speed);
        let touch = px(content.run.player_radius + 6);
        let mut taken = Vec::new();
        for (k, item) in self.pickups.iter_mut().enumerate() {
            if item.to.is_none() {
                item.to = reach
                    .iter()
                    .filter(|(_, pos, r)| within(item.pos, *pos, *r))
                    .min_by_key(|(_, pos, _)| dist2(item.pos, *pos))
                    .map(|(s, _, _)| *s);
            }
            if let Some(seat) = item.to {
                let Some((_, pos, _)) = reach.iter().find(|(s, _, _)| *s == seat) else {
                    item.to = None;
                    continue;
                };
                let to = pos.sub(item.pos);
                if to.len() <= touch as i64 {
                    taken.push((k, seat));
                } else {
                    item.pos = item.pos.add(to.with_len(speed.min(to.len() as i32)));
                }
            }
        }
        let mut gone = Vec::new();
        for (k, seat) in taken {
            gone.push(k);
            let item = self.pickups[k].clone();
            self.take(content, seat, &item);
        }
        let mut k = 0;
        self.pickups.retain(|_| {
            let keep = !gone.contains(&k);
            k += 1;
            keep
        });
    }

    fn take(&mut self, content: &Content, seat: usize, item: &Pickup) {
        match item.kind {
            PickupKind::Xp => self.gain_xp(content, item.value),
            PickupKind::Gold => self.players[seat].record.gold += item.value,
            PickupKind::Ticket => self.players[seat].record.tickets += item.value,
            PickupKind::Health => {
                let max = self.max_hp(content, seat);
                let p = &mut self.players[seat];
                p.hp = (p.hp + max * content.run.health_cell_pct / 100).min(max);
            }
            PickupKind::Cache => self.open_cache(content, seat),
        }
    }

    pub fn gain_xp(&mut self, content: &Content, value: i32) {
        let pct = 100
            + self
                .players
                .iter()
                .map(|p| p.load.stats.xp_pct)
                .max()
                .unwrap_or(0);
        self.xp += (value * pct + 50) / 100;
        loop {
            let need = World::xp_needed(content, self.level);
            if self.xp < need {
                break;
            }
            self.xp -= need;
            self.level += 1;
            for p in self.players.iter_mut() {
                p.pending += 1;
            }
        }
    }

    // ---- downed players ------------------------------------------------------

    fn ghosts(&mut self, content: &Content) {
        for seat in 0..self.players.len() {
            if self.players[seat].alive {
                continue;
            }
            let ghost = self.players[seat].pos;
            self.players[seat].ghost_drop -= 1;
            if self.players[seat].ghost_drop <= 0 && self.players.iter().any(|p| p.alive) {
                self.players[seat].ghost_drop = ticks(content.run.ghost_drop_ms);
                self.drop(content, PickupKind::Health, ghost, 1);
            }
            let helped = self
                .players
                .iter()
                .any(|p| p.alive && within(p.pos, ghost, px(content.run.revive_radius)));
            let p = &mut self.players[seat];
            if helped {
                p.revive_progress += 1;
            } else {
                p.revive_progress = (p.revive_progress - 2).max(0);
            }
            if p.revive_progress >= ticks(content.run.revive_ms) {
                for q in self.players.iter_mut() {
                    if q.alive && within(q.pos, ghost, px(content.run.revive_radius)) {
                        q.record.revives += 1;
                    }
                }
                let p = &mut self.players[seat];
                p.alive = true;
                p.revive_progress = 0;
                p.invulnerable = ticks(2000);
                let max = self.max_hp(content, seat);
                self.players[seat].hp = max * content.run.revive_hp_pct / 100;
                self.say("banner.revived", 2000);
            }
        }
    }

    // ---- rifts ---------------------------------------------------------------

    fn rift_tick(&mut self, content: &Content) {
        if self.in_rift {
            self.rift_left -= 1;
            if self.rift_left <= 0 {
                self.leave_rift(content, false);
            }
            return;
        }
        let Some(r) = self.rift.as_mut() else { return };
        r.left -= 1;
        if r.left <= 0 {
            self.rift = None;
            return;
        }
        let pos = r.pos;
        let reach = px(content.events.rift_radius + content.run.player_radius);
        if self.players.iter().any(|p| p.alive && within(p.pos, pos, reach)) {
            self.enter_rift(content);
        }
    }

    fn enter_rift(&mut self, content: &Content) {
        self.rift = None;
        self.stash = Some(Stash {
            foes: std::mem::take(&mut self.foes),
            bolts: std::mem::take(&mut self.bolts),
            pickups: std::mem::take(&mut self.pickups),
            effects: std::mem::take(&mut self.effects),
            positions: self.players.iter().map(|p| p.pos).collect(),
        });
        self.shots.clear();
        self.in_rift = true;
        self.fields.clear();
        self.pending.clear();
        self.rift_left = ticks(content.rift.time_limit_ms);
        let n = self.players.len() as i32;
        for (k, p) in self.players.iter_mut().enumerate() {
            p.pos = V::new(px(-200 + 60 * k as i32 - 30 * (n - 1)), px(120));
        }
        let kind = content.enemy_index(&content.rift.elite);
        self.spawn_foe(content, kind, V::new(px(200), px(-120)));
        self.say("banner.rift_enter", 2500);
    }

    fn rift_won(&mut self, content: &Content) {
        self.rifts_cleared += 1;
        for seat in 0..self.players.len() {
            let offer = self.reward_offer(content, seat);
            self.players[seat].offer = Some(offer);
            let extra = self.players[seat].load.stats.rift_tickets;
            self.players[seat].record.tickets += content.rift.tickets + extra;
        }
        self.leave_rift(content, true);
    }

    fn leave_rift(&mut self, _content: &Content, won: bool) {
        let Some(stash) = self.stash.take() else { return };
        self.in_rift = false;
        self.fields.clear();
        self.pending.clear();
        self.foes = stash.foes;
        self.bolts = stash.bolts;
        self.pickups = stash.pickups;
        self.effects = stash.effects;
        self.shots.clear();
        for (p, pos) in self.players.iter_mut().zip(stash.positions) {
            p.pos = pos;
            p.invulnerable = p.invulnerable.max(ticks(1500));
        }
        self.say(
            if won {
                "banner.rift_won"
            } else {
                "banner.rift_lost"
            },
            2500,
        );
    }

    // ---- housekeeping --------------------------------------------------------

    fn cull(&mut self, content: &Content) {
        let far = px(content.run.despawn_distance) as i64;
        let players: Vec<V> = self.players.iter().filter(|p| p.alive).map(|p| p.pos).collect();
        let mut relocate = Vec::new();
        for (i, f) in self.foes.iter().enumerate() {
            let e = &content.enemies[f.kind];
            if f.hp <= 0 || e.boss || e.elite || players.is_empty() {
                continue;
            }
            if players.iter().all(|p| dist2(*p, f.pos) > far * far) {
                relocate.push(i);
            }
        }
        // a straggler left far behind comes back in front, so the crowd
        // stays where the player is
        for i in relocate {
            let r = content.enemies[self.foes[i].kind].radius;
            let pos = self.spawn_point(content, r);
            self.foes[i].pos = pos;
        }
        self.foes.retain(|f| f.hp > 0);
    }

    fn check_end(&mut self, _content: &Content) {
        if self.phase == Phase::Playing && self.players.iter().all(|p| !p.alive) {
            self.phase = Phase::Lost;
        }
    }

    /// FNV-1a over the world's state, for the co-op desync check.
    pub fn sum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: i64| {
            h ^= v as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        mix(self.tick as i64);
        mix(self.xp as i64);
        mix(self.level as i64);
        mix(self.foes.len() as i64);
        for p in &self.players {
            mix(p.pos.x as i64);
            mix(p.pos.y as i64);
            mix(p.hp as i64);
        }
        for f in &self.foes {
            mix(f.pos.x as i64);
            mix(f.pos.y as i64);
            mix(f.hp as i64);
        }
        mix(self.shots.len() as i64);
        mix(self.pickups.len() as i64);
        h
    }
}

#[cfg(test)]
impl World {
    pub fn add_test_bolt(&mut self, pos: V) {
        self.bolts.push(Bolt {
            pos,
            vel: V::ZERO,
            damage: 1,
            radius: 8,
            life: 100,
        });
    }
}

pub fn shield_ticks(content: &Content, stats: &Stats) -> i32 {
    if stats.shield_s <= 0 {
        return 0;
    }
    ticks(1000 * (content.cache.shield_base_s - stats.shield_s).max(10))
}

/// A full turn, as the unsigned bound `Rng::below` takes.
pub fn geom_turn() -> u32 {
    crate::fx::TURN as u32
}
