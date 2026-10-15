//! What lasts between runs: gold, tickets, shop ranks, unlocked characters,
//! achievement progress, gear from the salvage cache, and settings. It
//! is all one `Save`, which the core writes as a JSON file for the page to
//! download and reads back from a file the player chooses, as in Gear
//! Master 2D. The page keeps nothing in browser storage (D17).

use crate::content::{Achievement, Content, Rarity};
use crate::rng::Rng;
use crate::world::{Loadout, Phase, Stats, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SAVE_FORMAT: &str = "galactic-survivors-save";
pub const SAVE_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub autoaim: bool,
    pub reduced_motion: bool,
    pub damage_numbers: bool,
}

/// One module the player owns. Two of the same module can share an id;
/// `uid` tells them apart (T8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub uid: u32,
    pub id: String,
    pub rank: i32,
}

/// A version 1 module: one per id, duplicates folded into its rank.
#[derive(Debug, Clone, Deserialize)]
struct OwnedV1 {
    id: String,
    rank: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    pub format: String,
    pub version: u32,
    pub gold: i32,
    pub tickets: i32,
    pub ranks: BTreeMap<String, i32>,
    pub unlocked: Vec<String>,
    /// Lifetime numbers the achievements read: totals and bests.
    pub stats: BTreeMap<String, u64>,
    pub claimed: Vec<String>,
    pub modules: Vec<Piece>,
    pub next_uid: u32,
    /// Gear per character: one uid per slot in `gear.slots`, 0 for empty.
    pub equipped: BTreeMap<String, Vec<u32>>,
    pub pity_rare: u32,
    pub pity_epic: u32,
    pub rng: Rng,
    pub settings: Settings,
    pub character: String,
    /// Rare parts pulled from the salvage cache (T15).
    #[serde(default)]
    pub parts: Vec<String>,
    /// The name a co-op partner sees (T24); empty until the player types one.
    #[serde(default)]
    pub name: String,
}

/// What a finished run paid, for the results screen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payout {
    pub won: bool,
    pub seconds: i32,
    pub kills: u32,
    pub level: i32,
    pub gold: i32,
    pub tickets: i32,
    pub achievements: Vec<String>,
}

/// One pull from the salvage cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pull {
    pub id: String,
    pub rarity: Rarity,
    pub rank: i32,
    pub new: bool,
    /// A rare part for rigs rather than a module.
    #[serde(default)]
    pub part: bool,
}

impl Save {
    pub fn new(content: &Content, seed: u64) -> Save {
        let unlocked = content
            .characters
            .iter()
            .filter(|c| c.unlock_gold == 0 && content.unlocked_by("character", &c.id).is_none())
            .map(|c| c.id.clone())
            .collect();
        Save {
            format: SAVE_FORMAT.into(),
            version: SAVE_VERSION,
            gold: 0,
            tickets: 0,
            ranks: BTreeMap::new(),
            unlocked,
            stats: BTreeMap::new(),
            claimed: Vec::new(),
            modules: Vec::new(),
            next_uid: 1,
            equipped: BTreeMap::new(),
            pity_rare: 0,
            pity_epic: 0,
            rng: Rng::new(seed ^ 0x5eed_ca5e),
            settings: Settings::default(),
            character: content.characters[0].id.clone(),
            parts: Vec::new(),
            name: String::new(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a save always serialises")
    }

    /// Reads a save file. A file from another game or a later version is
    /// refused with the key of a string that says which. A version 1 file
    /// is brought up to version 2 (`from_v1`).
    pub fn from_json(content: &Content, text: &str) -> Result<Save, &'static str> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|_| "save.not_json")?;
        if value.get("format").and_then(|f| f.as_str()) != Some(SAVE_FORMAT) {
            return Err("save.wrong_game");
        }
        let version = value.get("version").and_then(|v| v.as_u64());
        let mut save = if version == Some(1) {
            Save::from_v1(content, value)?
        } else if version == Some(SAVE_VERSION as u64) {
            serde_json::from_value(value).map_err(|_| "save.damaged")?
        } else {
            return Err("save.wrong_version");
        };
        // names that no longer exist in the content are dropped, not trusted
        save.unlocked.retain(|c| content.character(c).is_some());
        save.modules.retain(|m| content.module(&m.id).is_some());
        save.claimed.retain(|a| content.achievement(a).is_some());
        save.ranks
            .retain(|id, _| content.shop.iter().any(|s| &s.id == id));
        let uids: Vec<u32> = save.modules.iter().map(|m| m.uid).collect();
        let slots = content.gear.slots.len();
        for list in save.equipped.values_mut() {
            list.resize(slots, 0);
            for u in list.iter_mut() {
                if !uids.contains(u) {
                    *u = 0;
                }
            }
        }
        save.next_uid = save.next_uid.max(uids.iter().max().copied().unwrap_or(0) + 1);
        if content.character(&save.character).is_none() || !save.unlocked.contains(&save.character) {
            save.character = content.characters[0].id.clone();
        }
        save.claim(content);
        Ok(save)
    }

    /// Version 1 kept one module per id, two slots a character, and
    /// outfitter ranks with no caps. Each module becomes one piece at its
    /// rank and keeps its place where a slot of its kind is open; outfitter
    /// ranks are refunded at today's prices so they can be bought again
    /// under the caps (T3).
    fn from_v1(content: &Content, mut value: serde_json::Value) -> Result<Save, &'static str> {
        let obj = value.as_object_mut().ok_or("save.damaged")?;
        let old_modules: Vec<OwnedV1> = obj
            .get("modules")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| "save.damaged")?
            .unwrap_or_default();
        let old_equipped: BTreeMap<String, Vec<String>> = obj
            .get("equipped")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| "save.damaged")?
            .unwrap_or_default();
        let old_ranks: BTreeMap<String, i32> = obj
            .get("ranks")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| "save.damaged")?
            .unwrap_or_default();
        obj.insert("version".into(), SAVE_VERSION.into());
        obj.insert("modules".into(), serde_json::Value::Array(Vec::new()));
        obj.insert("next_uid".into(), 1.into());
        obj.insert("equipped".into(), serde_json::Value::Object(Default::default()));
        obj.insert("ranks".into(), serde_json::Value::Object(Default::default()));
        let mut save: Save = serde_json::from_value(value).map_err(|_| "save.damaged")?;
        save.claimed.retain(|a| content.achievement(a).is_some());
        for (id, rank) in &old_ranks {
            for r in 0..*rank {
                save.gold += Save::rank_cost(content, id, r).unwrap_or(0);
            }
        }
        for m in old_modules {
            if content.module(&m.id).is_some() {
                save.add_piece(&m.id, m.rank.clamp(1, content.cache.max_rank));
            }
        }
        for (ch, ids) in old_equipped {
            for id in ids {
                let Some(uid) = save.modules.iter().find(|m| m.id == id).map(|m| m.uid) else {
                    continue;
                };
                // a v1 module keeps its place only where a slot is free
                let _ = save.equip_auto(content, &ch, uid);
            }
        }
        Ok(save)
    }

    pub fn stat(&self, key: &str) -> u64 {
        self.stats.get(key).copied().unwrap_or(0)
    }

    fn add_stat(&mut self, key: &str, v: u64) {
        *self.stats.entry(key.to_string()).or_insert(0) += v;
    }

    fn best_stat(&mut self, key: &str, v: u64) {
        let e = self.stats.entry(key.to_string()).or_insert(0);
        *e = (*e).max(v);
    }

    /// Keeps the lowest nonzero value (a fastest time).
    fn low_stat(&mut self, key: &str, v: u64) {
        if v == 0 {
            return;
        }
        let e = self.stats.entry(key.to_string()).or_insert(0);
        if *e == 0 || v < *e {
            *e = v;
        }
    }

    // ---- what achievements unlock ------------------------------------------

    /// Whether `kind:id` is still locked: some achievement unlocks it and
    /// that achievement is not earned.
    pub fn locked(&self, content: &Content, kind: &str, id: &str) -> bool {
        if kind == "part" {
            // a rare part opens by its achievement or from the salvage cache
            let rare = content.part(id).map(|p| p.rare).unwrap_or(false);
            if !rare || self.parts.iter().any(|p| p == id) {
                return false;
            }
            return content
                .unlocked_by(kind, id)
                .map(|a| !self.claimed.contains(&a.id))
                .unwrap_or(true);
        }
        content
            .unlocked_by(kind, id)
            .map(|a| !self.claimed.contains(&a.id))
            .unwrap_or(false)
    }

    /// How many ranks of an outfitter line are for sale now: the ranks
    /// open from the start plus one for each earned achievement that opens
    /// one (T3).
    pub fn rank_cap(&self, content: &Content, id: &str) -> i32 {
        let Some(item) = content.shop.iter().find(|s| s.id == id) else {
            return 0;
        };
        let opened = content
            .achievements
            .iter()
            .filter(|a| a.unlocks() == Some(("rank", id)) && self.claimed.contains(&a.id))
            .count() as i32;
        (item.open + opened).min(item.ranks)
    }

    /// Whether gear slot `k` is open.
    pub fn slot_open(&self, content: &Content, k: usize) -> bool {
        k < content.gear.slots.len()
            && (k < content.gear.open_slots || !self.locked(content, "slot", &k.to_string()))
    }

    // ---- loadout -------------------------------------------------------------

    /// The pieces in a character's gear slots, by slot.
    pub fn gear_of(&self, content: &Content, character: &str) -> Vec<Option<&Piece>> {
        let list = self.equipped.get(character);
        (0..content.gear.slots.len())
            .map(|k| {
                let uid = list.and_then(|l| l.get(k)).copied().unwrap_or(0);
                if uid == 0 || !self.slot_open(content, k) {
                    return None;
                }
                self.modules.iter().find(|m| m.uid == uid)
            })
            .collect()
    }

    /// The stats this save brings into a run with a character: outfitter
    /// ranks, the character's gear, and banishes from achievements.
    pub fn loadout(&self, content: &Content, character: &str) -> Loadout {
        let mut stats = Stats::default();
        for item in &content.shop {
            let rank = self.ranks.get(&item.id).copied().unwrap_or(0);
            stats.add(&item.stat, item.per_rank * rank);
        }
        for piece in self.gear_of(content, character).into_iter().flatten() {
            if let Some(def) = content.module(&piece.id) {
                stats.add(&def.stat, def.per_rank * piece.rank);
            }
        }
        for a in &content.achievements {
            if let Some(("banish", n)) = a.unlocks() {
                if self.claimed.contains(&a.id) {
                    stats.banishes += n.parse::<i32>().unwrap_or(0);
                }
            }
        }
        let locked = content
            .abilities
            .iter()
            .filter(|a| self.locked(content, "ability", &a.id))
            .chain(
                content
                    .evolutions
                    .iter()
                    .filter(|e| self.locked(content, "evolution", &e.id)),
            )
            .map(|a| a.id.clone())
            .chain(
                content
                    .parts
                    .iter()
                    .filter(|p| self.locked(content, "part", &p.id))
                    .map(|p| p.id.clone()),
            )
            .collect();
        Loadout {
            character: character.to_string(),
            stats,
            autoaim: self.settings.autoaim,
            locked,
        }
    }

    // ---- the shop ------------------------------------------------------------

    pub fn rank_cost(content: &Content, id: &str, rank: i32) -> Option<i32> {
        let item = content.shop.iter().find(|s| s.id == id)?;
        if rank >= item.ranks {
            return None;
        }
        Some(item.cost + item.cost_step * rank)
    }

    pub fn buy_rank(&mut self, content: &Content, id: &str) -> Result<(), &'static str> {
        let rank = self.ranks.get(id).copied().unwrap_or(0);
        let cost = Save::rank_cost(content, id, rank).ok_or("shop.maxed")?;
        if rank >= self.rank_cap(content, id) {
            return Err("shop.capped");
        }
        if self.gold < cost {
            return Err("shop.short");
        }
        self.gold -= cost;
        self.ranks.insert(id.to_string(), rank + 1);
        self.add_stat("shop_ranks", 1);
        self.claim(content);
        Ok(())
    }

    pub fn buy_character(&mut self, content: &Content, id: &str) -> Result<(), &'static str> {
        let c = content.character(id).ok_or("shop.unknown")?;
        if self.unlocked.contains(&c.id) || c.unlock_gold <= 0 {
            return Err("shop.unknown");
        }
        if self.gold < c.unlock_gold {
            return Err("shop.short");
        }
        self.gold -= c.unlock_gold;
        self.unlocked.push(c.id.clone());
        Ok(())
    }

    // ---- achievements --------------------------------------------------------

    fn met(&self, a: &Achievement) -> bool {
        let v = self.stat(&a.stat);
        if a.at_most {
            v > 0 && v <= a.target
        } else {
            v >= a.target
        }
    }

    /// Progress toward an achievement as (now, target). A fastest-time
    /// achievement shows the best time so far.
    pub fn progress(&self, content: &Content, id: &str) -> (u64, u64) {
        let Some(a) = content.achievement(id) else {
            return (0, 1);
        };
        let v = self.stat(&a.stat);
        if a.at_most {
            (v, a.target)
        } else {
            (v.min(a.target), a.target)
        }
    }

    /// Whether the grid shows what an achievement asks: it is a starting
    /// one, it is earned, or an earned one sits next to it (T4). Hidden
    /// ones can still be earned.
    pub fn visible(&self, content: &Content, id: &str) -> bool {
        let Some(a) = content.achievement(id) else {
            return false;
        };
        if a.start || self.claimed.contains(&a.id) {
            return true;
        }
        content.achievements.iter().any(|b| {
            let d = (b.at[0] - a.at[0]).abs() + (b.at[1] - a.at[1]).abs();
            d == 1 && self.claimed.contains(&b.id)
        })
    }

    /// Pays out every achievement whose target has been met and that has
    /// not paid yet, seen or hidden. Returns the ids paid now. Each pays
    /// once (I20).
    pub fn claim(&mut self, content: &Content) -> Vec<String> {
        let mut paid = Vec::new();
        for a in &content.achievements {
            if self.claimed.contains(&a.id) || !self.met(a) {
                continue;
            }
            self.claimed.push(a.id.clone());
            self.gold += a.gold;
            self.tickets += a.tickets;
            paid.push(a.id.clone());
            if let Some(("character", id)) = a.unlocks() {
                if !self.unlocked.iter().any(|u| u == id) {
                    self.unlocked.push(id.to_string());
                }
            }
        }
        paid
    }

    /// Adds a finished run to the save: gold kept whether the run was won or
    /// lost (I15, I17), and the numbers the achievements read.
    pub fn finish_run(&mut self, content: &Content, world: &World, seat: usize) -> Payout {
        let r = &content.run;
        let p = &world.players[seat];
        let won = world.phase == Phase::Won;
        let seconds = world.seconds();
        let bosses = world.bosses_killed.len() as i32;
        let mut gold = p.record.gold
            + r.payout_per_minute * seconds / 60
            + p.record.kills as i32 / r.kills_per_gold.max(1)
            + r.payout_boss * bosses;
        if won {
            gold += r.payout_win;
        }
        let tickets = p.record.tickets;
        self.gold += gold;
        self.tickets += tickets;
        let ch = content.characters[p.character].id.clone();
        let tps = crate::geom::TPS;
        let untouched = if p.record.first_hit == 0 {
            seconds
        } else {
            p.record.first_hit / tps
        };
        let maxed = p
            .abilities
            .iter()
            .filter(|s| content.is_evolution(&s.id) || s.level >= r.max_level)
            .count();
        let bare = self.gear_of(content, &ch).iter().all(|g| g.is_none());
        self.add_stat("runs", 1);
        self.add_stat("kills", p.record.kills as u64);
        self.add_stat("time_s", seconds as u64);
        self.add_stat("dashes", p.record.dashes as u64);
        self.add_stat("revives", p.record.revives as u64);
        self.best_stat("best_kills", p.record.kills as u64);
        self.best_stat("best_time_s", seconds as u64);
        self.best_stat("best_level", world.level as u64);
        self.best_stat("best_untouched_s", untouched as u64);
        self.best_stat("best_gold_run", gold as u64);
        self.best_stat("best_rifts", world.rifts_cleared as u64);
        self.best_stat("best_evolutions", p.record.evolved.len() as u64);
        self.best_stat("best_maxed", maxed as u64);
        self.best_stat("best_full_slots", (p.abilities.len() + p.passives.len()) as u64);
        self.low_stat("fastest_evolve_s", (p.record.first_evolve / tps) as u64);
        self.add_stat("rifts", world.rifts_cleared as u64);
        let runes: u32 = p.record.runes.values().sum();
        self.add_stat("runes", runes as u64);
        self.best_stat("best_runes", runes as u64);
        self.best_stat("best_field_kills", p.record.field_kills as u64);
        self.best_stat("best_reactions", p.record.reactions as u64);
        self.best_stat("best_trigger_depth", p.record.deepest_trigger as u64);
        self.best_stat("best_rig_parts", p.record.fullest_rig as u64);
        self.add_stat("evolutions", p.record.evolved.len() as u64);
        for (ability, n) in &p.record.kills_by {
            self.best_stat(&format!("best_kills.{ability}"), *n as u64);
        }
        for (enemy, n) in &p.record.kills_of {
            self.add_stat(&format!("kills.{enemy}"), *n as u64);
        }
        for e in &p.record.evolved {
            self.add_stat(&format!("evolved.{e}"), 1);
        }
        for b in &world.bosses_killed {
            self.add_stat(&format!("boss.{b}"), 1);
        }
        if won {
            self.add_stat("wins", 1);
            self.add_stat(&format!("won.{ch}"), 1);
            if bare {
                self.add_stat("won_bare", 1);
            }
        }
        if world.coop {
            self.add_stat("coop_runs", 1);
            if won {
                self.add_stat("coop_wins", 1);
            }
        }
        let achievements = self.claim(content);
        Payout {
            won,
            seconds,
            kills: p.record.kills,
            level: world.level,
            gold,
            tickets,
            achievements,
        }
    }

    // ---- the salvage cache ---------------------------------------------------

    fn roll_rarity(&mut self, content: &Content) -> Rarity {
        let c = &content.cache;
        // pity: the tenth pull without a rare is at least rare; the
        // fortieth without an epic is epic
        if self.pity_epic + 1 >= c.pity_epic {
            return Rarity::Epic;
        }
        let total = c.common + c.rare + c.epic;
        let roll = self.rng.below(total);
        let rolled = if roll < c.epic {
            Rarity::Epic
        } else if roll < c.epic + c.rare {
            Rarity::Rare
        } else {
            Rarity::Common
        };
        if rolled == Rarity::Common && self.pity_rare + 1 >= c.pity_rare {
            return Rarity::Rare;
        }
        rolled
    }

    fn add_piece(&mut self, id: &str, rank: i32) -> u32 {
        let uid = self.next_uid.max(1);
        self.next_uid = uid + 1;
        self.modules.push(Piece {
            uid,
            id: id.to_string(),
            rank,
        });
        uid
    }

    fn count_owned(&mut self) {
        let mut ids: Vec<&str> = self.modules.iter().map(|m| m.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        let n = ids.len() as u64;
        let best = self.modules.iter().map(|m| m.rank).max().unwrap_or(0) as u64;
        self.best_stat("modules_owned", n);
        self.best_stat("best_module_rank", best);
    }

    /// Pulls from the salvage cache. Every pull is a new piece at rank 1;
    /// pieces rise in rank by merging (T8).
    pub fn pull(&mut self, content: &Content, count: i32) -> Result<Vec<Pull>, &'static str> {
        let cost = if count >= 10 {
            content.cache.ten_pull
        } else {
            content.cache.pull_cost * count
        };
        if self.tickets < cost {
            return Err("cache.short");
        }
        self.tickets -= cost;
        let mut out = Vec::new();
        for _ in 0..count {
            let rarity = self.roll_rarity(content);
            self.pity_rare = if rarity >= Rarity::Rare {
                0
            } else {
                self.pity_rare + 1
            };
            self.pity_epic = if rarity == Rarity::Epic {
                0
            } else {
                self.pity_epic + 1
            };
            // an epic pull may be a rare part not yet opened (T15)
            let closed: Vec<String> = content
                .parts
                .iter()
                .filter(|p| self.locked(content, "part", &p.id))
                .map(|p| p.id.clone())
                .collect();
            if rarity == Rarity::Epic && !closed.is_empty() && self.rng.chance(content.cache.part_pct, 100) {
                let id = closed[self.rng.below(closed.len() as u32) as usize].clone();
                self.parts.push(id.clone());
                self.add_stat("pulls", 1);
                self.add_stat("epic_pulls", 1);
                out.push(Pull {
                    id,
                    rarity,
                    rank: 0,
                    new: true,
                    part: true,
                });
                continue;
            }
            let pool: Vec<&str> = content
                .modules
                .iter()
                .filter(|m| m.rarity == rarity)
                .map(|m| m.id.as_str())
                .collect();
            let id = pool[self.rng.below(pool.len() as u32) as usize].to_string();
            let new = !self.modules.iter().any(|m| m.id == id);
            self.add_piece(&id, 1);
            self.add_stat("pulls", 1);
            if rarity == Rarity::Epic {
                self.add_stat("epic_pulls", 1);
            }
            out.push(Pull {
                id,
                rarity,
                rank: 1,
                new,
                part: false,
            });
        }
        self.count_owned();
        self.claim(content);
        Ok(out)
    }

    // ---- gear ------------------------------------------------------------------

    pub fn piece(&self, uid: u32) -> Option<&Piece> {
        self.modules.iter().find(|m| m.uid == uid)
    }

    /// The characters wearing a piece.
    pub fn worn_by(&self, uid: u32) -> Vec<String> {
        self.equipped
            .iter()
            .filter(|(_, l)| l.contains(&uid))
            .map(|(c, _)| c.clone())
            .collect()
    }

    /// Puts a piece in gear slot `k` of a character, in place of whatever
    /// was there. The slot must be open and of the piece's kind.
    pub fn equip(
        &mut self,
        content: &Content,
        character: &str,
        k: usize,
        uid: u32,
    ) -> Result<(), &'static str> {
        let def = self
            .piece(uid)
            .and_then(|p| content.module(&p.id))
            .ok_or("gear.unknown")?;
        if content.character(character).is_none() || k >= content.gear.slots.len() {
            return Err("gear.unknown");
        }
        if content.gear.slots[k] != def.slot {
            return Err("gear.wrong_slot");
        }
        if !self.slot_open(content, k) {
            return Err("gear.slot_locked");
        }
        let n = content.gear.slots.len();
        let list = self.equipped.entry(character.to_string()).or_default();
        list.resize(n, 0);
        for u in list.iter_mut() {
            if *u == uid {
                *u = 0;
            }
        }
        list[k] = uid;
        Ok(())
    }

    /// Equips a piece in the first open slot of its kind, an empty one if
    /// there is one.
    pub fn equip_auto(&mut self, content: &Content, character: &str, uid: u32) -> Result<(), &'static str> {
        let def = self
            .piece(uid)
            .and_then(|p| content.module(&p.id))
            .ok_or("gear.unknown")?;
        let gear = self.gear_of(content, character);
        let fits: Vec<usize> = (0..content.gear.slots.len())
            .filter(|&k| content.gear.slots[k] == def.slot && self.slot_open(content, k))
            .collect();
        let k = fits
            .iter()
            .copied()
            .find(|&k| gear[k].is_none())
            .or(fits.first().copied())
            .ok_or("gear.slot_locked")?;
        self.equip(content, character, k, uid)
    }

    pub fn unequip(&mut self, character: &str, uid: u32) {
        if let Some(list) = self.equipped.get_mut(character) {
            for u in list.iter_mut() {
                if *u == uid {
                    *u = 0;
                }
            }
        }
    }

    fn remove_piece(&mut self, uid: u32) {
        self.modules.retain(|m| m.uid != uid);
        for list in self.equipped.values_mut() {
            for u in list.iter_mut() {
                if *u == uid {
                    *u = 0;
                }
            }
        }
    }

    /// The other pieces a merge into `uid` would use: the same module at
    /// the same rank, unworn ones first.
    pub fn merge_partners(&self, content: &Content, uid: u32) -> Vec<u32> {
        let Some(me) = self.piece(uid) else {
            return Vec::new();
        };
        let mut others: Vec<&Piece> = self
            .modules
            .iter()
            .filter(|m| m.uid != uid && m.id == me.id && m.rank == me.rank)
            .collect();
        others.sort_by_key(|m| (!self.worn_by(m.uid).is_empty(), m.uid));
        others
            .into_iter()
            .take(content.gear.merge_count - 1)
            .map(|m| m.uid)
            .collect()
    }

    /// Merges copies of a piece into it: `merge_count` of the same module at
    /// the same rank become one, a rank higher, that stays where `uid` was
    /// worn.
    pub fn merge(&mut self, content: &Content, uid: u32) -> Result<(), &'static str> {
        let me = self.piece(uid).ok_or("gear.unknown")?.clone();
        if me.rank >= content.cache.max_rank {
            return Err("gear.max_rank");
        }
        let partners = self.merge_partners(content, uid);
        if partners.len() + 1 < content.gear.merge_count {
            return Err("gear.no_pair");
        }
        for u in partners {
            self.remove_piece(u);
        }
        if let Some(m) = self.modules.iter_mut().find(|m| m.uid == uid) {
            m.rank += 1;
        }
        self.add_stat("merges", 1);
        self.count_owned();
        self.claim(content);
        Ok(())
    }

    /// Merges every set that can merge, lowest ranks first, until none can.
    /// Returns how many merges ran.
    pub fn merge_all(&mut self, content: &Content) -> usize {
        let mut n = 0;
        loop {
            let mut order: Vec<&Piece> = self.modules.iter().collect();
            // a worn piece survives its merge, so it goes first
            order.sort_by_key(|m| (m.rank, self.worn_by(m.uid).is_empty(), m.uid));
            let next = order.iter().map(|m| m.uid).find(|&u| {
                self.piece(u)
                    .map(|p| p.rank < content.cache.max_rank)
                    .unwrap_or(false)
                    && self.merge_partners(content, u).len() + 1 >= content.gear.merge_count
            });
            match next {
                Some(u) if self.merge(content, u).is_ok() => n += 1,
                _ => return n,
            }
        }
    }

    /// Scraps an unworn piece for gold: the rarity's price at rank 1,
    /// doubled for each rank above it.
    pub fn scrap(&mut self, content: &Content, uid: u32) -> Result<i32, &'static str> {
        let me = self.piece(uid).ok_or("gear.unknown")?.clone();
        if !self.worn_by(uid).is_empty() {
            return Err("gear.worn");
        }
        let def = content.module(&me.id).ok_or("gear.unknown")?;
        let g = &content.gear;
        let base = match def.rarity {
            Rarity::Common => g.scrap_common,
            Rarity::Rare => g.scrap_rare,
            Rarity::Epic => g.scrap_epic,
        };
        let gold = base << (me.rank - 1).clamp(0, 16);
        self.remove_piece(uid);
        self.gold += gold;
        Ok(gold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::shipped;

    fn piece(s: &mut Save, id: &str, rank: i32) -> u32 {
        s.add_piece(id, rank)
    }

    #[test]
    fn a_save_round_trips() {
        let c = shipped();
        let mut s = Save::new(&c, 7);
        s.gold = 321;
        s.stats.insert("kills".into(), 44);
        piece(&mut s, "kinetic_core", 2);
        let back = Save::from_json(&c, &s.to_json()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_save_from_another_game_is_refused_with_a_reason() {
        let c = shipped();
        assert_eq!(
            Save::from_json(&c, "{\"format\": \"gm2d-save\", \"version\": 1}"),
            Err("save.wrong_game")
        );
        assert_eq!(Save::from_json(&c, "nonsense"), Err("save.not_json"));
        let later = Save::new(&c, 1)
            .to_json()
            .replace("\"version\": 2", "\"version\": 9");
        assert!(later.contains("\"version\": 9"), "the edit matched");
        assert_eq!(Save::from_json(&c, &later), Err("save.wrong_version"));
    }

    /// A version 1 file from the first build: one kinetic core at rank 2
    /// worn by Pathfinder, two plating ranks, and a module that no longer
    /// exists.
    #[test]
    fn a_version_1_save_is_brought_up_to_date() {
        let c = shipped();
        let v1 = r#"{"format": "galactic-survivors-save", "version": 1, "gold": 5, "tickets": 3,
            "ranks": {"plating_rank": 2}, "unlocked": ["pathfinder"], "stats": {},
            "claimed": ["not_an_achievement"],
            "modules": [{"id": "kinetic_core", "rank": 2}, {"id": "gone_mod", "rank": 1}],
            "equipped": {"pathfinder": ["kinetic_core"]}, "pity_rare": 4, "pity_epic": 4,
            "rng": {"state": 99}, "settings": {"autoaim": true, "reduced_motion": false, "damage_numbers": false},
            "character": "pathfinder"}"#;
        let s = Save::from_json(&c, v1).unwrap();
        assert_eq!(s.version, 2);
        // plating ranks refunded at today's prices: 120 for rank 1, 240 for rank 2
        assert!(s.ranks.is_empty());
        assert_eq!(s.gold, 5 + 120 + 240);
        assert_eq!(s.modules.len(), 1);
        assert_eq!((s.modules[0].id.as_str(), s.modules[0].rank), ("kinetic_core", 2));
        // the core slot is slot 0
        assert_eq!(s.equipped["pathfinder"][0], s.modules[0].uid);
        assert!(s.claimed.is_empty());
        assert_eq!(s.tickets, 3);
        // kinetic core: 3% a rank
        assert_eq!(s.loadout(&c, "pathfinder").stats.damage_pct, 6);
    }

    #[test]
    fn the_grid_shows_starting_achievements_and_the_neighbours_of_earned_ones() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        // first_steps starts visible at (3, 3); half_way is two cells left
        // at (1, 3), veteran between them at (2, 3)
        assert!(s.visible(&c, "first_steps"));
        assert!(!s.visible(&c, "veteran"));
        assert!(!s.visible(&c, "half_way"));
        s.stats.insert("best_time_s".into(), 180);
        s.claim(&c);
        assert!(s.visible(&c, "veteran"));
        assert!(!s.visible(&c, "half_way"));
    }

    #[test]
    fn a_hidden_achievement_is_earned_all_the_same() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        assert!(!s.visible(&c, "marathon"));
        s.stats.insert("time_s".into(), 7200);
        assert_eq!(s.claim(&c), vec!["marathon".to_string()]);
        assert!(s.visible(&c, "marathon"));
        // marathon pays 250 gold and 3 tickets in content.json
        assert_eq!((s.gold, s.tickets), (250, 3));
        assert!(s.claim(&c).is_empty());
    }

    #[test]
    fn a_fastest_time_achievement_needs_a_time_at_or_under_its_target() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        s.claim(&c);
        assert!(
            !s.claimed.contains(&"quick_study".to_string()),
            "never evolved is not fast"
        );
        s.low_stat("fastest_evolve_s", 250);
        s.low_stat("fastest_evolve_s", 400);
        assert_eq!(s.stat("fastest_evolve_s"), 250);
        s.claim(&c);
        // quick_study: 300 s or less; prodigy: 210 s or less
        assert!(s.claimed.contains(&"quick_study".to_string()));
        assert!(!s.claimed.contains(&"prodigy".to_string()));
    }

    #[test]
    fn an_achievement_unlocks_its_character() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        assert!(!s.unlocked.contains(&"vector".to_string()));
        s.stats.insert("boss.burrower".into(), 1);
        s.claim(&c);
        assert!(s.unlocked.contains(&"vector".to_string()));
    }

    #[test]
    fn locked_abilities_and_evolutions_stay_out_until_their_achievement() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        let locked = s.loadout(&c, "pathfinder").locked;
        for id in ["mortar", "missiles", "cryo_burst", "sentry", "storm_lattice"] {
            assert!(locked.contains(&id.to_string()), "{id}");
        }
        assert!(!locked.contains(&"chain_arc".to_string()));
        s.claimed.push("lightning_rod".into());
        s.claimed.push("rift_runner".into());
        let locked = s.loadout(&c, "pathfinder").locked;
        assert!(!locked.contains(&"storm_lattice".to_string()));
        assert!(!locked.contains(&"mortar".to_string()));
        assert!(locked.contains(&"sentry".to_string()));
    }

    #[test]
    fn first_evolution_adds_a_banish() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        assert_eq!(s.loadout(&c, "pathfinder").stats.banishes, 0);
        s.claimed.push("first_evolution".into());
        assert_eq!(s.loadout(&c, "pathfinder").stats.banishes, 1);
    }

    #[test]
    fn achievements_no_longer_add_xp() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        s.claimed = c.achievements.iter().map(|a| a.id.clone()).collect();
        assert_eq!(s.loadout(&c, "pathfinder").stats.xp_pct, 0);
    }

    #[test]
    fn outfitter_ranks_stop_at_the_cap_until_an_achievement_opens_more() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        s.gold = 10_000;
        // plating_rank: 1 rank open; 120 gold, then 240
        s.buy_rank(&c, "plating_rank").unwrap();
        assert_eq!(s.gold, 10_000 - 120);
        assert_eq!(s.buy_rank(&c, "plating_rank"), Err("shop.capped"));
        // shell_rank: none open at the start
        assert_eq!(s.buy_rank(&c, "shell_rank"), Err("shop.capped"));
        // first_steps opens a second plating rank
        s.stats.insert("best_time_s".into(), 180);
        s.claim(&c);
        let gold = s.gold;
        s.buy_rank(&c, "plating_rank").unwrap();
        assert_eq!(s.gold, gold - 240);
        assert_eq!(s.loadout(&c, "pathfinder").stats.max_hp, 20);
        assert_eq!(s.rank_cap(&c, "plating_rank"), 2);
    }

    #[test]
    fn every_outfitter_rank_can_be_opened_by_some_achievement() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        s.claimed = c.achievements.iter().map(|a| a.id.clone()).collect();
        for item in &c.shop {
            assert_eq!(s.rank_cap(&c, &item.id), item.ranks, "{}", item.id);
        }
    }

    #[test]
    fn ten_pulls_hold_a_rare_or_better() {
        let c = shipped();
        for seed in 0..200 {
            let mut s = Save::new(&c, seed);
            s.tickets = 10;
            let pulls = s.pull(&c, 10).unwrap();
            assert_eq!(pulls.len(), 10);
            assert!(pulls.iter().any(|p| p.rarity >= Rarity::Rare), "seed {seed}");
            // ten tickets spent; only achievements earned by the pull pay any back
            let back: i32 = c
                .achievements
                .iter()
                .filter(|a| s.claimed.contains(&a.id))
                .map(|a| a.tickets)
                .sum();
            assert_eq!(s.tickets, back);
            assert_eq!(
                s.modules.len() + s.parts.len(),
                10,
                "every pull is its own piece or a rare part"
            );
        }
    }

    #[test]
    fn an_epic_pull_can_open_a_rare_part_once_and_it_joins_the_cards() {
        let c = shipped();
        let rare: Vec<&str> = c.parts.iter().filter(|p| p.rare).map(|p| p.id.as_str()).collect();
        let mut s = Save::new(&c, 9);
        s.tickets = 100_000;
        for _ in 0..400 {
            s.pull(&c, 10).unwrap();
        }
        // every rare part was pulled, none twice
        let mut got = s.parts.clone();
        got.sort();
        let mut want: Vec<String> = rare.iter().map(|r| r.to_string()).collect();
        want.sort();
        assert_eq!(got, want);
        let load = s.loadout(&c, "pathfinder");
        for r in rare {
            assert!(!load.locked.iter().any(|l| l == r), "{r} is offered once pulled");
        }
        let fresh = Save::new(&c, 9).loadout(&c, "pathfinder");
        assert!(
            fresh.locked.iter().any(|l| l == "overcharge"),
            "a new save has not opened it"
        );
        assert!(
            !fresh.locked.iter().any(|l| l == "ignite"),
            "a common part is never locked"
        );
    }

    #[test]
    fn a_pull_without_tickets_is_refused() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        assert_eq!(s.pull(&c, 1), Err("cache.short"));
    }

    #[test]
    fn two_copies_at_one_rank_merge_into_one_a_rank_higher() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        // the rank 2 copy comes before the second rank 1 copy, so a merge
        // that ignored rank would take it first
        let a = piece(&mut s, "kinetic_core", 1);
        let odd = piece(&mut s, "kinetic_core", 2);
        let b = piece(&mut s, "kinetic_core", 1);
        let other = piece(&mut s, "coolant_mod", 1);
        s.merge(&c, a).unwrap();
        assert_eq!(s.piece(a).unwrap().rank, 2);
        assert!(s.piece(b).is_none(), "the copy was used up");
        assert!(s.piece(odd).is_some() && s.piece(other).is_some());
        // a lone coolant mod has nothing to merge with
        assert_eq!(s.merge(&c, other), Err("gear.no_pair"));
        // the two rank 2 kinetic cores make rank 3
        s.merge(&c, a).unwrap();
        assert_eq!(s.piece(a).unwrap().rank, 3);
        assert_eq!(s.modules.len(), 2);
        assert_eq!(s.stat("merges"), 2);
    }

    #[test]
    fn a_piece_at_the_highest_rank_does_not_merge() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        let a = piece(&mut s, "kinetic_core", c.cache.max_rank);
        piece(&mut s, "kinetic_core", c.cache.max_rank);
        assert_eq!(s.merge(&c, a), Err("gear.max_rank"));
    }

    #[test]
    fn merge_all_merges_upward_and_keeps_the_worn_piece() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        let ids: Vec<u32> = (0..4).map(|_| piece(&mut s, "plating_mod", 1)).collect();
        // wear the last one: plating_mod fits the frame slot, slot 1
        s.equip(&c, "pathfinder", 1, ids[3]).unwrap();
        // four rank 1 become two rank 2, then one rank 3
        assert_eq!(s.merge_all(&c), 3);
        assert_eq!(s.modules.len(), 1);
        assert_eq!(s.modules[0].uid, ids[3]);
        assert_eq!(s.modules[0].rank, 3);
        assert_eq!(s.equipped["pathfinder"][1], ids[3]);
    }

    #[test]
    fn gear_goes_only_in_open_slots_of_its_kind_and_counts_per_character() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        let core = piece(&mut s, "kinetic_core", 2);
        let frame = piece(&mut s, "plating_mod", 2);
        let util = piece(&mut s, "magnet_mod", 1);
        // slots: 0 core, 1 frame, 2 and 3 utility; 2 and 3 start locked
        assert_eq!(s.equip(&c, "pathfinder", 1, core), Err("gear.wrong_slot"));
        s.equip(&c, "pathfinder", 0, core).unwrap();
        s.equip_auto(&c, "pathfinder", frame).unwrap();
        assert_eq!(s.equip(&c, "pathfinder", 2, util), Err("gear.slot_locked"));
        assert_eq!(s.equip_auto(&c, "pathfinder", util), Err("gear.slot_locked"));
        s.claimed.push("long_haul".into());
        s.equip_auto(&c, "pathfinder", util).unwrap();
        assert_eq!(s.equipped["pathfinder"], vec![core, frame, util, 0]);
        let l = s.loadout(&c, "pathfinder");
        // kinetic_core 3% a rank, plating_mod 8 a rank, magnet_mod 10% a rank
        assert_eq!(
            (l.stats.damage_pct, l.stats.max_hp, l.stats.pickup_pct),
            (6, 16, 10)
        );
        assert_eq!(s.loadout(&c, "breaker").stats.damage_pct, 0);
    }

    #[test]
    fn scrapping_pays_by_rarity_and_rank_and_spares_worn_gear() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        let a = piece(&mut s, "kinetic_core", 2);
        let b = piece(&mut s, "aegis_core", 1);
        s.equip(&c, "pathfinder", 1, b).unwrap();
        assert_eq!(s.scrap(&c, b), Err("gear.worn"));
        // common: 10 gold at rank 1, doubled once for rank 2
        assert_eq!(s.scrap(&c, a), Ok(20));
        assert_eq!(s.gold, 20);
        assert!(s.piece(a).is_none());
    }
}
