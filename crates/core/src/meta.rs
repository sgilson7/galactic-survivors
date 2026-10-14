//! What lasts between runs: gold, tickets, shop ranks, unlocked characters,
//! achievement progress, modules from the salvage cache, and settings. It
//! is all one `Save`, which the core writes as a JSON file for the page to
//! download and reads back from a file the player chooses, as in Gear
//! Master 2D. The page keeps nothing in browser storage (D17).

use crate::content::{Content, Rarity};
use crate::rng::Rng;
use crate::world::{Loadout, Phase, Stats, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SAVE_FORMAT: &str = "galactic-survivors-save";
pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub autoaim: bool,
    pub reduced_motion: bool,
    pub damage_numbers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owned {
    pub id: String,
    pub rank: i32,
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
    pub modules: Vec<Owned>,
    /// Module ids equipped per character, at most `cache.slots` each.
    pub equipped: BTreeMap<String, Vec<String>>,
    pub pity_rare: u32,
    pub pity_epic: u32,
    pub rng: Rng,
    pub settings: Settings,
    pub character: String,
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
}

impl Save {
    pub fn new(content: &Content, seed: u64) -> Save {
        let unlocked = content
            .characters
            .iter()
            .filter(|c| c.unlock_gold == 0 && c.unlock_achievement.is_empty())
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
            equipped: BTreeMap::new(),
            pity_rare: 0,
            pity_epic: 0,
            rng: Rng::new(seed ^ 0x5eed_ca5e),
            settings: Settings::default(),
            character: content.characters[0].id.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a save always serialises")
    }

    /// Reads a save file. A file from another game or a later version is
    /// refused with the key of a string that says which.
    pub fn from_json(content: &Content, text: &str) -> Result<Save, &'static str> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|_| "save.not_json")?;
        if value.get("format").and_then(|f| f.as_str()) != Some(SAVE_FORMAT) {
            return Err("save.wrong_game");
        }
        if value.get("version").and_then(|v| v.as_u64()) != Some(SAVE_VERSION as u64) {
            return Err("save.wrong_version");
        }
        let mut save: Save = serde_json::from_value(value).map_err(|_| "save.damaged")?;
        // names that no longer exist in the content are dropped, not trusted
        save.unlocked.retain(|c| content.character(c).is_some());
        save.modules.retain(|m| content.module(&m.id).is_some());
        save.ranks
            .retain(|id, _| content.shop.iter().any(|s| &s.id == id));
        if content.character(&save.character).is_none() || !save.unlocked.contains(&save.character) {
            save.character = content.characters[0].id.clone();
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

    // ---- loadout -------------------------------------------------------------

    /// The stats this save brings into a run with a character: shop ranks,
    /// equipped modules and the achievement XP bonus.
    pub fn loadout(&self, content: &Content, character: &str) -> Loadout {
        let mut stats = Stats::default();
        for item in &content.shop {
            let rank = self.ranks.get(&item.id).copied().unwrap_or(0);
            stats.add(&item.stat, item.per_rank * rank);
        }
        for id in self.equipped.get(character).map(|v| v.as_slice()).unwrap_or(&[]) {
            if let (Some(def), Some(own)) = (content.module(id), self.modules.iter().find(|m| &m.id == id)) {
                stats.add(&def.stat, def.per_rank * own.rank);
            }
        }
        stats.xp_pct += content.achievement_xp_pct * self.claimed.len() as i32;
        for a in &content.achievements {
            if self.claimed.contains(&a.id) {
                stats.banishes += a.banishes;
            }
        }
        let locked = content
            .evolutions
            .iter()
            .filter(|e| !e.unlock_achievement.is_empty() && !self.claimed.contains(&e.unlock_achievement))
            .map(|e| e.id.clone())
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

    /// Progress toward an achievement, capped at its target.
    pub fn progress(&self, content: &Content, id: &str) -> (u64, u64) {
        let Some(a) = content.achievements.iter().find(|a| a.id == id) else {
            return (0, 1);
        };
        (self.stat(&a.stat).min(a.target), a.target)
    }

    /// Pays out every achievement whose target has been met and that has
    /// not paid yet. Returns the ids paid now. Each pays once (I20).
    pub fn claim(&mut self, content: &Content) -> Vec<String> {
        let mut paid = Vec::new();
        for a in &content.achievements {
            if self.claimed.contains(&a.id) || self.stat(&a.stat) < a.target {
                continue;
            }
            self.claimed.push(a.id.clone());
            self.gold += a.gold;
            self.tickets += a.tickets;
            paid.push(a.id.clone());
            for c in &content.characters {
                if c.unlock_achievement == a.id && !self.unlocked.contains(&c.id) {
                    self.unlocked.push(c.id.clone());
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
            + r.payout_per_ten_kills * p.record.kills as i32 / 10
            + r.payout_boss * bosses;
        if won {
            gold += r.payout_win;
        }
        let tickets = p.record.tickets;
        self.gold += gold;
        self.tickets += tickets;
        let ch = content.characters[p.character].id.clone();
        self.add_stat("runs", 1);
        self.add_stat("kills", p.record.kills as u64);
        self.best_stat("best_kills", p.record.kills as u64);
        self.best_stat("best_time_s", seconds as u64);
        self.best_stat("best_level", world.level as u64);
        self.add_stat("rifts", world.rifts_cleared as u64);
        self.add_stat("evolutions", p.record.evolved.len() as u64);
        for (ability, n) in &p.record.kills_by {
            self.best_stat(&format!("best_kills.{ability}"), *n as u64);
        }
        for e in &p.record.evolved {
            self.add_stat(&format!("evolved.{e}"), 1);
        }
        for b in &world.bosses_killed {
            self.add_stat(&format!("boss.{b}"), 1);
        }
        if won {
            self.add_stat(&format!("won.{ch}"), 1);
        }
        if world.coop {
            self.add_stat("coop_runs", 1);
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
            let pool: Vec<&str> = content
                .modules
                .iter()
                .filter(|m| m.rarity == rarity)
                .map(|m| m.id.as_str())
                .collect();
            let id = pool[self.rng.below(pool.len() as u32) as usize].to_string();
            let max = content.cache.max_rank;
            let (rank, new) = match self.modules.iter_mut().find(|m| m.id == id) {
                Some(m) => {
                    m.rank = (m.rank + 1).min(max);
                    (m.rank, false)
                }
                None => {
                    self.modules.push(Owned {
                        id: id.clone(),
                        rank: 1,
                    });
                    (1, true)
                }
            };
            if !new && rank == max {
                // a duplicate of a finished module is returned as gold
                self.gold += content.cache.duplicate_gold;
            }
            self.add_stat("pulls", 1);
            if rarity == Rarity::Epic {
                self.add_stat("epic_pulls", 1);
            }
            out.push(Pull {
                id,
                rarity,
                rank,
                new,
            });
        }
        self.claim(content);
        Ok(out)
    }

    /// Puts a module in a character's slots, or takes it out if it is there.
    pub fn toggle_module(
        &mut self,
        content: &Content,
        character: &str,
        id: &str,
    ) -> Result<(), &'static str> {
        if !self.modules.iter().any(|m| m.id == id) || content.character(character).is_none() {
            return Err("cache.unknown");
        }
        let slots = content.cache.slots;
        let list = self.equipped.entry(character.to_string()).or_default();
        if let Some(k) = list.iter().position(|m| m == id) {
            list.remove(k);
            return Ok(());
        }
        if list.len() >= slots {
            return Err("cache.full");
        }
        list.push(id.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::shipped;

    #[test]
    fn a_save_round_trips() {
        let c = shipped();
        let mut s = Save::new(&c, 7);
        s.gold = 321;
        s.stats.insert("kills".into(), 44);
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
            .replace("\"version\": 1", "\"version\": 9");
        assert_eq!(Save::from_json(&c, &later), Err("save.wrong_version"));
    }

    #[test]
    fn achievements_are_listed_before_they_are_earned_and_pay_once() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        // every achievement has visible progress from the first launch
        for a in &c.achievements {
            assert_eq!(s.progress(&c, &a.id), (0, a.target));
        }
        s.stats.insert("kills".into(), 1000);
        let paid = s.claim(&c);
        assert_eq!(paid, vec!["exterminator".to_string()]);
        // exterminator pays 150 gold and 1 ticket in content.json
        assert_eq!((s.gold, s.tickets), (150, 1));
        assert!(s.claim(&c).is_empty());
        assert_eq!(s.gold, 150);
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
    fn each_claimed_achievement_adds_one_percent_xp() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        s.claimed = vec!["first_steps".into(), "exterminator".into()];
        assert_eq!(s.loadout(&c, "pathfinder").stats.xp_pct, 2);
    }

    #[test]
    fn storm_lattice_stays_locked_until_lightning_rod() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        assert_eq!(
            s.loadout(&c, "pathfinder").locked,
            vec!["storm_lattice".to_string()]
        );
        s.claimed.push("lightning_rod".into());
        assert!(s.loadout(&c, "pathfinder").locked.is_empty());
    }

    #[test]
    fn ranks_cost_more_each_time_and_stop_at_the_cap() {
        let c = shipped();
        let mut s = Save::new(&c, 1);
        s.gold = 10_000;
        // revive_rank: one rank at 500
        s.buy_rank(&c, "revive_rank").unwrap();
        assert_eq!(s.gold, 9_500);
        assert_eq!(s.buy_rank(&c, "revive_rank"), Err("shop.maxed"));
        // plating_rank: 80, then 140
        s.buy_rank(&c, "plating_rank").unwrap();
        s.buy_rank(&c, "plating_rank").unwrap();
        assert_eq!(s.gold, 9_500 - 80 - 140);
        assert_eq!(s.loadout(&c, "pathfinder").stats.max_hp, 20);
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
            assert_eq!(s.tickets, 0);
        }
    }

    #[test]
    fn a_duplicate_raises_rank_up_to_the_cap() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        s.tickets = 400;
        s.pull(&c, 10).unwrap();
        for _ in 0..30 {
            s.pull(&c, 10).unwrap();
        }
        assert!(s
            .modules
            .iter()
            .all(|m| m.rank >= 1 && m.rank <= c.cache.max_rank));
        assert!(s.modules.iter().any(|m| m.rank == c.cache.max_rank));
    }

    #[test]
    fn a_pull_without_tickets_is_refused() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        assert_eq!(s.pull(&c, 1), Err("cache.short"));
    }

    #[test]
    fn modules_fill_two_slots_and_count_in_the_loadout() {
        let c = shipped();
        let mut s = Save::new(&c, 3);
        for id in ["kinetic_core", "plating_mod", "magnet_mod"] {
            s.modules.push(Owned {
                id: id.into(),
                rank: 2,
            });
        }
        s.toggle_module(&c, "pathfinder", "kinetic_core").unwrap();
        s.toggle_module(&c, "pathfinder", "plating_mod").unwrap();
        assert_eq!(s.toggle_module(&c, "pathfinder", "magnet_mod"), Err("cache.full"));
        let l = s.loadout(&c, "pathfinder");
        // kinetic_core 3% a rank, plating_mod 8 a rank, at rank 2
        assert_eq!((l.stats.damage_pct, l.stats.max_hp), (6, 16));
        assert_eq!(s.loadout(&c, "breaker").stats.damage_pct, 0);
    }
}
