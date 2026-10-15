//! Level-up and rift-reward cards. A level-up offers a few cards drawn from
//! what this character may take (I8, I14): its own abilities, the shared
//! ones, and passives. Passive regeneration does not exist, so no card can
//! offer it (I12). Rerolls draw a new hand; a banish takes a card out of
//! this run for good and draws a replacement (I10).

use crate::content::Content;
use crate::world::{Card, CardKind, Offer, OfferKind, Slot, World, CARD_REROLL};

impl World {
    /// Every card this player could be offered now.
    pub fn card_pool(&self, content: &Content, seat: usize) -> Vec<Card> {
        let p = &self.players[seat];
        let ch = &content.characters[p.character];
        let max = content.run.max_level;
        let mut pool = Vec::new();
        for slot in &p.abilities {
            if !content.is_evolution(&slot.id) && slot.level < max && !p.banished.contains(&slot.id) {
                pool.push(Card {
                    kind: CardKind::AbilityUp,
                    id: slot.id.clone(),
                    level: slot.level + 1,
                });
            }
        }
        if p.abilities.len() < content.run.ability_slots {
            for a in &content.abilities {
                let owned = p.abilities.iter().any(|s| {
                    s.id == a.id
                        || content
                            .ability(&s.id)
                            .map(|d| d.evolution == a.id)
                            .unwrap_or(false)
                });
                let starter_of_other = content
                    .characters
                    .iter()
                    .any(|c| c.starter == a.id && c.id != ch.id);
                let allowed = (a.owner.is_empty() || a.owner == ch.id) && !starter_of_other;
                if !owned && allowed && !p.banished.contains(&a.id) && !p.load.locked.contains(&a.id) {
                    pool.push(Card {
                        kind: CardKind::NewAbility,
                        id: a.id.clone(),
                        level: 1,
                    });
                }
            }
        }
        for slot in &p.passives {
            let def_max = content
                .passive(&slot.id)
                .map(|d| content.passive_max(d))
                .unwrap_or(max);
            if slot.level < def_max && !p.banished.contains(&slot.id) {
                pool.push(Card {
                    kind: CardKind::PassiveUp,
                    id: slot.id.clone(),
                    level: slot.level + 1,
                });
            }
        }
        // parts are drawn on their own (`draw_cards`); see `part_pool`
        if p.passives.len() < content.run.passive_slots {
            for d in &content.passives {
                if !p.passives.iter().any(|s| s.id == d.id) && !p.banished.contains(&d.id) {
                    pool.push(Card {
                        kind: CardKind::NewPassive,
                        id: d.id.clone(),
                        level: 1,
                    });
                }
            }
        }
        pool
    }

    /// Every part this player could be offered: the common ones, and the
    /// rare ones their save has opened (T15).
    pub fn part_pool(&self, content: &Content, seat: usize) -> Vec<Card> {
        let p = &self.players[seat];
        content
            .parts
            .iter()
            .filter(|d| !p.banished.contains(&d.id) && !p.load.locked.contains(&d.id))
            .map(|d| Card {
                kind: CardKind::Part,
                id: d.id.clone(),
                level: 1,
            })
            .collect()
    }

    fn draw_cards(&mut self, content: &Content, seat: usize, keep: &[Card]) -> Vec<Card> {
        let mut pool: Vec<Card> = self
            .card_pool(content, seat)
            .into_iter()
            .filter(|c| !keep.iter().any(|k| k.id == c.id))
            .collect();
        let mut hand = keep.to_vec();
        // one card raises something already held, when one can: with many
        // more abilities, passives and parts (T14, T15) a held ability's next
        // level was rarely offered and evolutions all but stopped
        if hand.len() < content.run.card_count && self.rng.chance(content.run.up_card_pct, 100) {
            let ups: Vec<usize> = pool
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c.kind, CardKind::AbilityUp | CardKind::PassiveUp))
                .map(|(k, _)| k)
                .collect();
            if !ups.is_empty() {
                let k = ups[self.rng.below(ups.len() as u32) as usize];
                hand.push(pool.remove(k));
            }
        }
        // at most one part a hand, part_card_pct of the time
        let parts: Vec<Card> = self
            .part_pool(content, seat)
            .into_iter()
            .filter(|c| !keep.iter().any(|k| k.id == c.id))
            .collect();
        if !parts.is_empty()
            && hand.len() < content.run.card_count
            && !keep.iter().any(|k| k.kind == CardKind::Part)
            && self.rng.chance(content.run.part_card_pct, 100)
        {
            let k = self.rng.below(parts.len() as u32) as usize;
            hand.push(parts[k].clone());
        }
        while hand.len() < content.run.card_count && !pool.is_empty() {
            let k = self.rng.below(pool.len() as u32) as usize;
            hand.push(pool.remove(k));
        }
        if hand.is_empty() {
            hand.push(Card {
                kind: CardKind::Gold,
                id: String::new(),
                level: 0,
            });
        }
        hand
    }

    /// A level-up hand. When an ability can evolve, the hand holds the
    /// evolution as its first card, so an evolution never waits on a
    /// supply cache (I9: evolutions should come earlier).
    pub fn level_offer(&mut self, content: &Content, seat: usize) -> Offer {
        let mut keep = Vec::new();
        if let Some(slot) = self.evolvable(content, seat) {
            let id = self.players[seat].abilities[slot].id.clone();
            let into = content
                .ability(&id)
                .map(|d| d.evolution.clone())
                .unwrap_or_default();
            keep.push(Card {
                kind: CardKind::Evolve,
                id: into,
                level: 1,
            });
        }
        let cards = self.draw_cards(content, seat, &keep);
        Offer {
            kind: OfferKind::LevelUp,
            cards,
        }
    }

    /// The reward for clearing a rift: evolve now if possible (or raise an
    /// ability), repair, or salvage.
    pub fn reward_offer(&mut self, content: &Content, seat: usize) -> Offer {
        let mut cards = Vec::new();
        if let Some(slot) = self.evolvable(content, seat) {
            let id = self.players[seat].abilities[slot].id.clone();
            let into = content
                .ability(&id)
                .map(|d| d.evolution.clone())
                .unwrap_or_default();
            cards.push(Card {
                kind: CardKind::Evolve,
                id: into,
                level: 1,
            });
        } else {
            let ups: Vec<Card> = self
                .card_pool(content, seat)
                .into_iter()
                .filter(|c| c.kind == CardKind::AbilityUp)
                .collect();
            if !ups.is_empty() {
                let k = self.rng.below(ups.len() as u32) as usize;
                cards.push(ups[k].clone());
            }
        }
        cards.push(Card {
            kind: CardKind::Heal,
            id: String::new(),
            level: content.rift.heal_pct,
        });
        cards.push(Card {
            kind: CardKind::Salvage,
            id: String::new(),
            level: content.rift.gold,
        });
        Offer {
            kind: OfferKind::Reward,
            cards,
        }
    }

    /// A card action from a seat's input: take, banish or reroll.
    pub fn card_action(&mut self, content: &Content, seat: usize, action: u8) {
        let Some(offer) = self.players[seat].offer.clone() else {
            return;
        };
        match action {
            1..=3 => {
                let k = (action - 1) as usize;
                let Some(card) = offer.cards.get(k).cloned() else {
                    return;
                };
                self.players[seat].offer = None;
                self.apply_card(content, seat, &card);
            }
            4..=6 if offer.kind == OfferKind::LevelUp => {
                let k = (action - 4) as usize;
                let p = &self.players[seat];
                let Some(card) = offer.cards.get(k).cloned() else {
                    return;
                };
                if p.banishes <= 0 || card.id.is_empty() {
                    return;
                }
                let p = &mut self.players[seat];
                p.banishes -= 1;
                p.banished.push(card.id.clone());
                let keep: Vec<Card> = offer.cards.iter().filter(|c| c.id != card.id).cloned().collect();
                let hand = self.draw_cards(content, seat, &keep);
                self.players[seat].offer = Some(Offer {
                    kind: OfferKind::LevelUp,
                    cards: hand,
                });
            }
            CARD_REROLL if offer.kind == OfferKind::LevelUp => {
                if self.players[seat].rerolls <= 0 {
                    return;
                }
                self.players[seat].rerolls -= 1;
                let offer = self.level_offer(content, seat);
                self.players[seat].offer = Some(offer);
            }
            _ => {}
        }
    }

    fn apply_card(&mut self, content: &Content, seat: usize, card: &Card) {
        match card.kind {
            CardKind::NewAbility => {
                let start = if content.characters[self.players[seat].character].starter == card.id {
                    card.level
                } else {
                    1
                };
                self.players[seat].abilities.push(Slot {
                    id: card.id.clone(),
                    level: start,
                    cd: 30,
                });
                self.place_ability(content, seat, &card.id);
            }
            CardKind::AbilityUp => {
                if let Some(s) = self.players[seat].abilities.iter_mut().find(|s| s.id == card.id) {
                    s.level = card.level;
                }
            }
            CardKind::NewPassive => {
                self.players[seat].passives.push(Slot {
                    id: card.id.clone(),
                    level: 1,
                    cd: 0,
                });
                self.after_passive(content, seat);
                self.fit_rigs(content, seat);
            }
            CardKind::PassiveUp => {
                if let Some(s) = self.players[seat].passives.iter_mut().find(|s| s.id == card.id) {
                    s.level = card.level;
                }
                self.after_passive(content, seat);
                self.fit_rigs(content, seat);
            }
            CardKind::Gold => self.players[seat].record.gold += content.run.gold_chip * 5,
            CardKind::Evolve => {
                if let Some(slot) = self.evolvable(content, seat) {
                    self.evolve(content, seat, slot);
                }
            }
            CardKind::Heal => {
                let max = self.max_hp(content, seat);
                let p = &mut self.players[seat];
                if p.alive {
                    p.hp = (p.hp + max * card.level / 100).min(max);
                }
            }
            CardKind::Salvage => self.players[seat].record.gold += card.level,
            CardKind::Part => self.place_part(content, seat, &card.id),
        }
    }

    /// A max-health passive raises current health by the same amount.
    fn after_passive(&mut self, content: &Content, seat: usize) {
        let max = self.max_hp(content, seat);
        let p = &mut self.players[seat];
        if p.alive && p.hp < max {
            let gained = content.passive("plating").map(|d| d.per_level).unwrap_or(0);
            p.hp = (p.hp + gained).min(max);
        }
    }
}
