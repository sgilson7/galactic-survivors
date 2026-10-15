//! The game's content, read from `data/content.json`: characters, abilities,
//! enemies, the schedule of a run, the shop, achievements and modules. Every
//! rule number lives in that file; this module only gives it types and
//! checks that each reference names something that exists.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Obstacle {
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub r: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunRules {
    pub length_s: i32,
    pub map_half: i32,
    pub spawn_distance: i32,
    pub despawn_distance: i32,
    pub enemy_cap: usize,
    pub shot_cap: usize,
    pub effect_cap: usize,
    pub pickup_cap: usize,
    pub card_count: usize,
    pub rerolls: i32,
    pub banishes: i32,
    pub ability_slots: usize,
    pub passive_slots: usize,
    pub max_level: i32,
    /// The level the partner passive must reach before an ability can
    /// evolve (T12).
    pub evolve_partner_level: i32,
    pub contact_window_ms: i32,
    pub invulnerable_ms: i32,
    /// In co-op the run goes on while a player chooses a card; the chooser
    /// is shielded for at most this long (T13, the person's answer 1).
    #[serde(default)]
    pub choose_guard_ms: i32,
    /// Sockets in each rig before Extra Socket (T15).
    #[serde(default)]
    pub rig_sockets: i32,
    /// The chance, in percent, that a level-up hand holds a part card.
    #[serde(default)]
    pub part_card_pct: u32,
    /// The chance, in percent, that a hand holds one card raising
    /// something already held.
    #[serde(default)]
    pub up_card_pct: u32,
    pub hit_freeze_ms: i32,
    pub flash_ms: i32,
    pub knockback: i32,
    pub pickup_radius: i32,
    pub magnet_speed: i32,
    pub player_radius: i32,
    pub xp_first: i32,
    pub xp_step: i32,
    pub xp_late_level: i32,
    pub xp_late_step: i32,
    pub gold_drop_pct: u32,
    pub health_drop_pct: u32,
    pub health_cell_pct: i32,
    pub gold_chip: i32,
    pub payout_per_minute: i32,
    /// Enemies defeated per gold paid at the end of a run.
    pub kills_per_gold: i32,
    pub payout_boss: i32,
    pub payout_win: i32,
    pub hp_scale_per_minute_pct: i32,
    pub damage_scale_per_minute_pct: i32,
    pub ghost_drop_ms: i32,
    pub ghost_heal_pct: i32,
    pub revive_ms: i32,
    pub revive_radius: i32,
    pub revive_hp_pct: i32,
    pub coop_hp_pct: i32,
    pub coop_spawn_pct: i32,
    pub obstacles: Vec<Obstacle>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Character {
    pub id: String,
    pub starter: String,
    pub exclusive: Vec<String>,
    pub hp: i32,
    pub speed: i32,
    pub damage_pct: i32,
    pub armor: i32,
    #[serde(rename = "trait", default)]
    pub trait_: String,
    #[serde(default)]
    pub trait_levels: Vec<i32>,
    #[serde(default)]
    pub contact_taken_pct: i32,
    #[serde(default)]
    pub reflect: i32,
    #[serde(default)]
    pub dash_ms: i32,
    #[serde(default)]
    pub dash_speed: i32,
    #[serde(default)]
    pub dash_cooldown_ms: i32,
    pub unlock_gold: i32,
}

/// The numbers for one level of an ability. A behaviour reads the fields it
/// needs; the rest stay zero.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Level {
    pub damage: i32,
    pub cooldown_ms: i32,
    pub count: i32,
    pub speed: i32,
    pub radius: i32,
    pub pierce: i32,
    pub bounce: i32,
    pub homing: i32,
    pub spread: i32,
    pub range: i32,
    pub width: i32,
    pub duration_ms: i32,
    pub knockback: i32,
    pub pull: i32,
    pub slow_pct: i32,
    pub slow_ms: i32,
    pub fuse_ms: i32,
    pub tick_ms: i32,
    pub fire_ms: i32,
    /// A ground field the ability leaves where it lands (T14): `fire`,
    /// `frost`, `charge` or `slick`; empty for none.
    pub field: String,
    /// A disc that flies out and comes back (Boomerang Disc).
    pub returns: i32,
    /// Strikes land one after another around the circle instead of at once
    /// (Sweep Cannon).
    pub sweep: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Shot,
    Nova,
    Arc,
    Orbit,
    Chain,
    Bomb,
    Beam,
    Turret,
    /// Points marked on a circle around the origin, struck after a fuse
    /// (Orbital Lance, Sweep Cannon).
    Strike,
    /// A pit that pulls enemies in and hurts them (Gravity Well).
    Well,
    /// A line from the player to the partner, or to the player's farthest
    /// placed effect, that cuts what crosses it (Tether).
    Tether,
}

/// How an ability chooses where to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aim {
    /// The nearest enemy, boss or not.
    Nearest,
    /// Toward the pointer, or the nearest enemy when auto-aim is on.
    Pointer,
    /// The enemy among the nearest few with the most others close by.
    Cluster,
    /// The direction the player is moving.
    Move,
    /// A random enemy in range.
    Random,
    /// The player's own position.
    #[serde(rename = "self")]
    Own,
    /// Behind the player, and only while moving (Wake Mine).
    Behind,
}

/// What a part is (T15). A rig reads its sockets left to right: a
/// modifier changes the next payload, a formation fires the next payload
/// several times arranged, and a trigger fires the rest of the rig when its
/// event happens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartKind {
    #[default]
    Modifier,
    Formation,
    Trigger,
}

/// The event a trigger waits for. Hit, expire and kill belong to the
/// payload just before the trigger; the rest belong to the player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum On {
    #[default]
    Hit,
    Expire,
    Kill,
    Stride,
    Dash,
    Rune,
    Clock,
}

impl On {
    /// Whether the event belongs to the player rather than a payload.
    pub fn is_player(self) -> bool {
        matches!(self, On::Stride | On::Dash | On::Rune | On::Clock)
    }
}

/// How a formation lays out its copies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arrange {
    #[default]
    Twin,
    Fan,
    Ring,
    Behind,
    Mirror,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Part {
    pub id: String,
    pub kind: PartKind,
    /// Added to the rig's recharge, in percent.
    pub load_pct: i32,
    /// Opened by an achievement or the salvage cache, not offered before.
    pub rare: bool,
    // a modifier
    pub damage_pct: i32,
    pub area_pct: i32,
    pub speed_pct: i32,
    pub count: i32,
    pub bounce: i32,
    pub pierce: i32,
    pub homing: bool,
    pub field: String,
    pub slow_pct: i32,
    pub slow_ms: i32,
    pub pull: i32,
    /// The payload fires again this long after.
    pub echo_ms: i32,
    /// The payload starts at the farthest enemy in reach and aims at the player.
    pub wormhole: bool,
    /// A burst of this percent of the payload's damage where it ends.
    pub burst_pct: i32,
    // a formation
    pub arrange: Arrange,
    pub copies: i32,
    // a trigger
    pub on: On,
    pub stride_px: i32,
    pub every_ms: i32,
}

/// One kind of ground field.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FieldDef {
    pub ms: i32,
    pub radius: i32,
    pub max_radius: i32,
    pub tick_ms: i32,
    /// Damage each tick, as a percent of the payload that left it.
    pub damage_pct: i32,
    pub slow_pct: i32,
}

/// Ground fields and how they react (T14).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FieldRules {
    pub cap: usize,
    pub fire: FieldDef,
    pub frost: FieldDef,
    pub charge: FieldDef,
    pub slick: FieldDef,
    pub steam: FieldDef,
    /// Fire meeting slick: a burst of this percent of the fire's damage.
    pub flare_pct: i32,
    /// Charge meeting slick or frost: each enemy in it is struck for this
    /// percent of the charge's damage.
    pub conduct_pct: i32,
}

/// Runes drawn by walking (T14).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RuneRules {
    /// A point of the trail is kept every this many pixels walked.
    pub sample_px: i32,
    pub max_points: usize,
    pub min_points: usize,
    /// The smallest loop that counts, in square pixels.
    pub min_area: i64,
    /// A turn sharper than this many degrees, measured across two points,
    /// is a corner (cosine in thousandths).
    pub corner_cos: i64,
    pub ms: i32,
    pub stacks: i32,
    pub damage: i32,
    pub circle_armor: i32,
    pub triangle_damage_pct: i32,
    pub square_area_pct: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ability {
    pub id: String,
    pub kind: Kind,
    pub aim: Aim,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub evolves_with: String,
    #[serde(default)]
    pub evolution: String,
    pub levels: Vec<Level>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Passive {
    pub id: String,
    pub stat: String,
    pub per_level: i32,
    #[serde(default)]
    pub max_level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Behaviour {
    Chase,
    Ranged,
    Charger,
    Burst,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Enemy {
    pub id: String,
    pub hp: i32,
    pub speed: i32,
    pub damage: i32,
    pub radius: i32,
    pub xp: i32,
    pub behaviour: Behaviour,
    pub size: i32,
    #[serde(default)]
    pub keep: i32,
    #[serde(default)]
    pub fire_ms: i32,
    #[serde(default)]
    pub shot_speed: i32,
    #[serde(default)]
    pub shot_damage: i32,
    #[serde(default)]
    pub burst: i32,
    #[serde(default)]
    pub volley: i32,
    #[serde(default)]
    pub charge_ms: i32,
    #[serde(default)]
    pub charge_speed: i32,
    #[serde(default)]
    pub knock_resist_pct: i32,
    #[serde(default)]
    pub spawn: String,
    #[serde(default)]
    pub spawn_ms: i32,
    #[serde(default)]
    pub spawn_count: i32,
    #[serde(default)]
    pub elite: bool,
    #[serde(default)]
    pub boss: bool,
    #[serde(default, rename = "final")]
    pub is_final: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Wave {
    pub from_s: i32,
    pub to_s: i32,
    pub every_ms: i32,
    pub count: i32,
    pub enemy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Timed {
    pub at_s: i32,
    pub enemy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaidSpawn {
    pub every_ms: i32,
    pub count: i32,
    pub enemy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Raid {
    pub from_s: i32,
    pub to_s: i32,
    pub warning_ms: i32,
    pub length_ms: i32,
    pub boss: String,
    pub spawns: Vec<RaidSpawn>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Events {
    pub rifts_s: Vec<i32>,
    pub rift_open_ms: i32,
    pub rift_distance: i32,
    pub rift_radius: i32,
    pub minibosses: Vec<Timed>,
    pub raid: Raid,
    #[serde(rename = "final")]
    pub final_boss: Timed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiftRules {
    pub arena_radius: i32,
    pub elite: String,
    pub time_limit_ms: i32,
    pub heal_pct: i32,
    pub tickets: i32,
    pub gold: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShopItem {
    pub id: String,
    pub stat: String,
    pub per_rank: i32,
    pub ranks: i32,
    /// Ranks for sale from the start; achievements open the rest (T3).
    pub open: i32,
    pub cost: i32,
    pub cost_step: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub stat: String,
    pub target: u64,
    /// Met when the stat is at most the target (a fastest time) rather
    /// than at least it. A stat of 0 means never done.
    #[serde(default)]
    pub at_most: bool,
    /// Column and row on the achievement grid (T4).
    pub at: [i32; 2],
    /// Visible from the first launch; the rest show once a neighbour is
    /// earned.
    #[serde(default)]
    pub start: bool,
    /// Picture under `icons/ach_<icon>`.
    pub icon: String,
    pub gold: i32,
    pub tickets: i32,
    /// What it unlocks, as `kind:id`: `character:`, `ability:`,
    /// `evolution:`, `rank:` (one more outfitter rank), `slot:` (a gear
    /// slot by number) or `banish:` (a count). Empty for none.
    #[serde(default)]
    pub unlock: String,
}

/// The kinds an achievement may unlock.
pub const UNLOCKS: &[&str] = &[
    "character",
    "ability",
    "evolution",
    "rank",
    "slot",
    "banish",
    "part",
];

impl Achievement {
    /// The unlock split into kind and id, if any.
    pub fn unlocks(&self) -> Option<(&str, &str)> {
        self.unlock.split_once(':')
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rarity {
    Common,
    Rare,
    Epic,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub id: String,
    pub rarity: Rarity,
    /// Which kind of gear slot it fits (`GearRules::slots`).
    pub slot: String,
    pub stat: String,
    pub per_rank: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheRules {
    pub pull_cost: i32,
    pub ten_pull: i32,
    pub common: u32,
    pub rare: u32,
    pub epic: u32,
    pub pity_rare: u32,
    pub pity_epic: u32,
    pub max_rank: i32,
    pub shield_base_s: i32,
    /// The chance, in percent, that an epic pull is a rare part the save
    /// has not opened yet, while one is left (T15).
    #[serde(default)]
    pub part_pct: u32,
}

/// The gear window (T8, T9): typed slots, merging and scrapping.
#[derive(Debug, Clone, Deserialize)]
pub struct GearRules {
    /// The kind of each slot, in order. A module fits a slot of its kind.
    pub slots: Vec<String>,
    /// How many slots are open before any achievement opens more.
    pub open_slots: usize,
    /// Copies of the same module at the same rank that merge into one
    /// rank higher.
    pub merge_count: usize,
    /// Gold for scrapping a module at rank 1, by rarity; it doubles with
    /// each rank.
    pub scrap_common: i32,
    pub scrap_rare: i32,
    pub scrap_epic: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Content {
    pub run: RunRules,
    pub characters: Vec<Character>,
    pub abilities: Vec<Ability>,
    pub evolutions: Vec<Ability>,
    pub passives: Vec<Passive>,
    pub enemies: Vec<Enemy>,
    pub waves: Vec<Wave>,
    pub events: Events,
    pub rift: RiftRules,
    pub shop: Vec<ShopItem>,
    pub achievements: Vec<Achievement>,
    pub modules: Vec<Module>,
    pub cache: CacheRules,
    pub gear: GearRules,
    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default)]
    pub fields: FieldRules,
    #[serde(default)]
    pub runes: RuneRules,
}

/// The stats a passive, a shop rank or a module may change. A name outside
/// this list in the content file is refused at load.
pub const STATS: &[&str] = &[
    "damage_pct",
    "cooldown_pct",
    "area_pct",
    "projectiles",
    "speed_pct",
    "max_hp",
    "armor",
    "pickup_pct",
    "rerolls",
    "banishes",
    "revives",
    "starter_levels",
    "shield_s",
    "rift_tickets",
    "momentum_pct",
    "anchor_pct",
    "rune_level",
    "heat_sink_pct",
    "double_tap",
    "fuse_area_pct",
    "sockets",
    "formation_extra",
];

/// The ground fields a payload or part may name.
pub const FIELDS: &[&str] = &["fire", "frost", "charge", "slick"];

/// Stats the person ruled out: "xp and gold: not allowed". No shop item or
/// module may raise them.
pub const FORBIDDEN_STATS: &[&str] = &["xp_pct", "gold_pct"];

impl Content {
    pub fn parse(json: &str) -> Result<Content, String> {
        let c: Content = serde_json::from_str(json).map_err(|e| format!("content.json: {e}"))?;
        c.check()?;
        Ok(c)
    }

    pub fn character(&self, id: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.id == id)
    }

    /// An ability or an evolution, by id.
    pub fn ability(&self, id: &str) -> Option<&Ability> {
        self.abilities
            .iter()
            .chain(self.evolutions.iter())
            .find(|a| a.id == id)
    }

    pub fn passive(&self, id: &str) -> Option<&Passive> {
        self.passives.iter().find(|p| p.id == id)
    }

    pub fn enemy(&self, id: &str) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.id == id)
    }

    pub fn enemy_index(&self, id: &str) -> usize {
        self.enemies.iter().position(|e| e.id == id).unwrap_or(0)
    }

    pub fn part(&self, id: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.id == id)
    }

    pub fn module(&self, id: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == id)
    }

    pub fn is_evolution(&self, id: &str) -> bool {
        self.evolutions.iter().any(|a| a.id == id)
    }

    /// The achievement that unlocks `kind:id`, if one does.
    pub fn unlocked_by(&self, kind: &str, id: &str) -> Option<&Achievement> {
        self.achievements.iter().find(|a| a.unlocks() == Some((kind, id)))
    }

    pub fn achievement(&self, id: &str) -> Option<&Achievement> {
        self.achievements.iter().find(|a| a.id == id)
    }

    /// The level a partner passive must reach for its ability to evolve:
    /// the run's rule, or the passive's own highest level if that is lower.
    pub fn partner_level(&self, passive: &str) -> i32 {
        let max = self.passive(passive).map(|p| self.passive_max(p)).unwrap_or(1);
        self.run.evolve_partner_level.min(max).max(1)
    }

    pub fn passive_max(&self, p: &Passive) -> i32 {
        if p.max_level > 0 {
            p.max_level
        } else {
            self.run.max_level
        }
    }

    fn check(&self) -> Result<(), String> {
        let ability_known = |id: &str| self.ability(id).is_some();
        for c in &self.characters {
            if !ability_known(&c.starter) {
                return Err(format!(
                    "character {} starts with unknown ability {}",
                    c.id, c.starter
                ));
            }
            for x in &c.exclusive {
                if !ability_known(x) {
                    return Err(format!("character {} lists unknown ability {x}", c.id));
                }
            }
        }
        for a in self.abilities.iter().chain(self.evolutions.iter()) {
            if a.levels.is_empty() {
                return Err(format!("ability {} has no levels", a.id));
            }
            if !a.evolves_with.is_empty() && self.passive(&a.evolves_with).is_none() {
                return Err(format!(
                    "ability {} evolves with unknown passive {}",
                    a.id, a.evolves_with
                ));
            }
            if !a.evolution.is_empty() && !self.is_evolution(&a.evolution) {
                return Err(format!("ability {} evolves into unknown {}", a.id, a.evolution));
            }
            if !a.owner.is_empty() && self.character(&a.owner).is_none() {
                return Err(format!(
                    "ability {} belongs to unknown character {}",
                    a.id, a.owner
                ));
            }
            if a.levels.iter().any(|l| l.cooldown_ms <= 0) {
                return Err(format!("ability {} has a level with no cooldown", a.id));
            }
        }
        for a in &self.abilities {
            let levels = a.levels.len() as i32;
            if levels != self.run.max_level {
                return Err(format!("ability {} has {levels} levels", a.id));
            }
        }
        let stat_ok = |s: &str| STATS.contains(&s) && !FORBIDDEN_STATS.contains(&s);
        for p in &self.passives {
            if !stat_ok(&p.stat) {
                return Err(format!("passive {} raises unknown stat {}", p.id, p.stat));
            }
        }
        for s in &self.shop {
            if !stat_ok(&s.stat) {
                return Err(format!("shop item {} raises a stat it may not: {}", s.id, s.stat));
            }
        }
        for m in &self.modules {
            if !stat_ok(&m.stat) {
                return Err(format!("module {} raises a stat it may not: {}", m.id, m.stat));
            }
        }
        let enemy_known = |id: &str| self.enemy(id).is_some();
        for w in &self.waves {
            if !enemy_known(&w.enemy) || w.every_ms <= 0 {
                return Err(format!(
                    "a wave names unknown enemy {} or has no interval",
                    w.enemy
                ));
            }
        }
        for t in self
            .events
            .minibosses
            .iter()
            .chain(std::iter::once(&self.events.final_boss))
        {
            if !enemy_known(&t.enemy) {
                return Err(format!("the schedule names unknown enemy {}", t.enemy));
            }
        }
        if !enemy_known(&self.events.raid.boss) || !enemy_known(&self.rift.elite) {
            return Err("the raid or the rift names an unknown enemy".into());
        }
        for s in &self.events.raid.spawns {
            if !enemy_known(&s.enemy) || s.every_ms <= 0 {
                return Err(format!("the raid names unknown enemy {}", s.enemy));
            }
        }
        for e in &self.enemies {
            if !e.spawn.is_empty() && !enemy_known(&e.spawn) {
                return Err(format!("enemy {} spawns unknown {}", e.id, e.spawn));
            }
        }
        for m in &self.modules {
            if !self.gear.slots.contains(&m.slot) {
                return Err(format!("module {} fits no gear slot: {}", m.id, m.slot));
            }
        }
        if self.gear.open_slots > self.gear.slots.len() || self.gear.merge_count < 2 {
            return Err("the gear rules open more slots than exist or merge fewer than two".into());
        }
        let mut cells = std::collections::BTreeSet::new();
        for a in &self.achievements {
            if !cells.insert(a.at) {
                return Err(format!("achievement {} shares its grid cell", a.id));
            }
            if a.target == 0 {
                return Err(format!("achievement {} has no target", a.id));
            }
            let Some((kind, id)) = a.unlocks() else {
                if a.unlock.is_empty() {
                    continue;
                }
                return Err(format!("achievement {} unlocks \"{}\"", a.id, a.unlock));
            };
            let known = match kind {
                "character" => self.character(id).is_some(),
                "ability" => self.abilities.iter().any(|x| x.id == id),
                "evolution" => self.is_evolution(id),
                "rank" => self.shop.iter().any(|s| s.id == id),
                "slot" => id
                    .parse::<usize>()
                    .map(|n| n >= self.gear.open_slots && n < self.gear.slots.len())
                    .unwrap_or(false),
                "banish" => id.parse::<i32>().is_ok(),
                "part" => self.part(id).map(|p| p.rare).unwrap_or(false),
                _ => false,
            };
            if !known || !UNLOCKS.contains(&kind) {
                return Err(format!("achievement {} unlocks unknown {}", a.id, a.unlock));
            }
        }
        if !self.achievements.iter().any(|a| a.start) {
            return Err("no achievement is visible from the start".into());
        }
        for s in &self.shop {
            let more = self
                .achievements
                .iter()
                .filter(|a| a.unlocks() == Some(("rank", s.id.as_str())))
                .count() as i32;
            if s.open + more != s.ranks {
                return Err(format!(
                    "outfitter line {} has {} ranks but {} open and {more} from achievements",
                    s.id, s.ranks, s.open
                ));
            }
        }
        for p in &self.parts {
            if self.ability(&p.id).is_some() || self.passive(&p.id).is_some() {
                return Err(format!("part {} shares its id with an ability or passive", p.id));
            }
            if !p.field.is_empty() && !FIELDS.contains(&p.field.as_str()) {
                return Err(format!("part {} leaves unknown field {}", p.id, p.field));
            }
            if p.kind == PartKind::Formation && p.copies < 1 {
                return Err(format!("formation {} makes no copies", p.id));
            }
            if p.kind == PartKind::Trigger
                && ((p.on == On::Stride && p.stride_px <= 0) || (p.on == On::Clock && p.every_ms <= 0))
            {
                return Err(format!("trigger {} has no stride or interval", p.id));
            }
        }
        for a in self.abilities.iter().chain(self.evolutions.iter()) {
            for l in &a.levels {
                if !l.field.is_empty() && !FIELDS.contains(&l.field.as_str()) {
                    return Err(format!("ability {} leaves unknown field {}", a.id, l.field));
                }
            }
        }
        if self.cache.common + self.cache.rare + self.cache.epic == 0 {
            return Err("the salvage cache has no odds".into());
        }
        Ok(())
    }
}

/// The content file the game ships with, for tests.
#[cfg(test)]
pub fn shipped() -> Content {
    Content::parse(include_str!("../../../data/content.json")).expect("shipped content loads")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_file_loads_and_every_reference_resolves() {
        let c = shipped();
        assert_eq!(c.characters.len(), 3);
        assert!(c.achievements.len() >= 50, "T5 asks for many more achievements");
        for ch in &c.characters {
            assert!(c.ability(&ch.starter).is_some());
        }
    }

    #[test]
    fn a_shop_item_that_raises_xp_or_gold_is_refused() {
        let text = include_str!("../../../data/content.json").replacen(
            "\"stat\": \"max_hp\", \"per_rank\": 10",
            "\"stat\": \"xp_pct\", \"per_rank\": 10",
            1,
        );
        assert!(text.contains("xp_pct"), "the edit matched");
        let err = Content::parse(&text).unwrap_err();
        assert!(err.contains("may not"), "{err}");
    }

    #[test]
    fn the_shop_sells_no_xp_or_gold_bonus() {
        let c = shipped();
        for s in &c.shop {
            assert!(!FORBIDDEN_STATS.contains(&s.stat.as_str()), "{}", s.id);
        }
        for m in &c.modules {
            assert!(!FORBIDDEN_STATS.contains(&m.stat.as_str()), "{}", m.id);
        }
    }

    #[test]
    fn an_unknown_reference_is_refused() {
        let text = include_str!("../../../data/content.json").replacen(
            "\"starter\": \"seeker_rifle\"",
            "\"starter\": \"nothing_here\"",
            1,
        );
        assert!(Content::parse(&text).is_err());
    }
}
