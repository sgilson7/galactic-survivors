//! The screen as a view model. The page builds the DOM from this without
//! knowing what a panel means: panels hold items and actions, and an action
//! is a name and value the page sends back through `act`. Actions named
//! `page.*` ask the page to do input or output the core cannot (download the
//! save, read a file, open the transport); the page does that and reports
//! back through `act`.

use crate::content::{Kind, Rarity};
use crate::game::{Game, Screen};
use crate::online;
use crate::world::{CardKind, OfferKind};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Action {
    pub name: String,
    pub value: String,
    pub label: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub primary: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hint: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Item {
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tag: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tone: String,
    /// Percent toward a goal, for a progress bar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// Column and row on a grid panel, from 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<[i32; 2]>,
    /// A short mark in the corner of a tile.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub badge: String,
    /// What clicking the whole item does, for tiles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<Action>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub selected: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Panel {
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub heading: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Item>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Ui {
    pub screen: String,
    pub title: String,
    pub subtitle: String,
    pub panels: Vec<Panel>,
    /// Shown over the canvas during a run: cards, pause, waiting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay: Option<Panel>,
    /// The run's build strip under the canvas.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub build: Vec<Item>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notice: String,
    pub reduced_motion: bool,
}

fn act(name: &str, value: &str, label: String) -> Action {
    Action {
        name: name.into(),
        value: value.into(),
        label,
        enabled: true,
        ..Action::default()
    }
}

fn primary(mut a: Action) -> Action {
    a.primary = true;
    a
}

fn enabled(mut a: Action, on: bool) -> Action {
    a.enabled = on;
    a
}

fn hint(mut a: Action, h: String) -> Action {
    a.hint = h;
    a
}

fn seconds_text(ms: i32) -> String {
    format!("{}.{}", ms / 1000, (ms % 1000) / 100)
}

fn back(g: &Game) -> Action {
    act("screen.show", "title", g.text.get("menu.back"))
}

pub fn build(g: &Game) -> Ui {
    let t = &g.text;
    let mut ui = Ui {
        screen: g.screen.name().into(),
        title: t.get(&format!("screen.{}.title", g.screen.name())),
        subtitle: t.get(&format!("screen.{}.subtitle", g.screen.name())),
        reduced_motion: g.save.settings.reduced_motion,
        ..Ui::default()
    };
    if !g.notice.is_empty() {
        ui.notice = t.get(&g.notice);
    }
    match g.screen {
        Screen::Title => title(g, &mut ui),
        Screen::Hangar => hangar(g, &mut ui),
        Screen::Shop => shop(g, &mut ui),
        Screen::Cache => cache(g, &mut ui),
        Screen::Achievements => achievements(g, &mut ui),
        Screen::Settings => settings(g, &mut ui),
        Screen::Coop => coop(g, &mut ui),
        Screen::Run => run(g, &mut ui),
        Screen::Results => results(g, &mut ui),
        Screen::Help => help(g, &mut ui),
        Screen::Gear => gear(g, &mut ui),
        Screen::Codex => {
            ui.panels.push(codex(g));
            ui.panels.push(Panel {
                id: "nav".into(),
                kind: "bar".into(),
                actions: vec![back(g)],
                ..Panel::default()
            });
        }
    }
    ui
}

fn purse(g: &Game) -> String {
    g.text.fill(
        "purse",
        &[
            ("gold", g.save.gold.to_string()),
            ("tickets", g.save.tickets.to_string()),
        ],
    )
}

fn title(g: &Game, ui: &mut Ui) {
    let t = &g.text;
    ui.panels.push(Panel {
        id: "menu".into(),
        kind: "menu".into(),
        actions: vec![
            primary(act("screen.show", "hangar", t.get("menu.solo"))),
            act("screen.show", "coop", t.get("menu.coop")),
            act("screen.show", "gear", t.get("menu.gear")),
            act("screen.show", "shop", t.get("menu.shop")),
            act("screen.show", "cache", t.get("menu.cache")),
            act("screen.show", "achievements", t.get("menu.achievements")),
            act("screen.show", "codex", t.get("menu.codex")),
            act("screen.show", "settings", t.get("menu.settings")),
            act("screen.show", "help", t.get("menu.help")),
        ],
        ..Panel::default()
    });
    ui.panels.push(save_panel(g));
}

fn save_panel(g: &Game) -> Panel {
    let t = &g.text;
    Panel {
        id: "save".into(),
        heading: t.get("save.heading"),
        text: format!("{}\n{}", purse(g), t.get("save.explain")),
        actions: vec![
            act("page.download", "", t.get("save.download")),
            act("page.load", "", t.get("save.load")),
        ],
        ..Panel::default()
    }
}

fn character_item(g: &Game, id: &str) -> Item {
    let c = &g.content;
    let t = &g.text;
    let ch = c.character(id).expect("known character");
    let unlocked = g.save.unlocked.iter().any(|u| u == id);
    let chosen = g.save.character == id;
    let mut actions = Vec::new();
    let mut tag = String::new();
    if unlocked {
        actions.push(enabled(
            act(
                "hangar.pick",
                id,
                t.get(if chosen { "hangar.chosen" } else { "hangar.pick" }),
            ),
            !chosen,
        ));
    } else if ch.unlock_gold > 0 {
        actions.push(enabled(
            act(
                "hangar.buy",
                id,
                t.fill("hangar.buy", &[("gold", ch.unlock_gold.to_string())]),
            ),
            g.save.gold >= ch.unlock_gold,
        ));
    } else {
        tag = unlock_hint(g, "character", id).unwrap_or_default();
    }
    Item {
        id: id.into(),
        icon: format!("players/{id}"),
        title: t.name(id),
        text: format!(
            "{} {}",
            t.get(&format!("about.{id}")),
            t.fill(
                "hangar.stats",
                &[
                    ("hp", ch.hp.to_string()),
                    ("speed", ch.speed.to_string()),
                    ("starter", t.name(&ch.starter)),
                ]
            )
        ),
        tag,
        tone: if chosen {
            "chosen".into()
        } else if unlocked {
            String::new()
        } else {
            "locked".into()
        },
        actions,
        ..Item::default()
    }
}

fn hangar(g: &Game, ui: &mut Ui) {
    let t = &g.text;
    let items = g
        .content
        .characters
        .iter()
        .map(|c| character_item(g, &c.id))
        .collect();
    ui.panels.push(Panel {
        id: "characters".into(),
        kind: "cards".into(),
        heading: t.get("hangar.heading"),
        text: purse(g),
        items,
        ..Panel::default()
    });
    ui.panels.push(Panel {
        id: "gear".into(),
        kind: "list".into(),
        heading: t.fill("gear.heading", &[("name", t.name(&g.save.character))]),
        text: gear_totals(g),
        items: doll_items(g, false),
        actions: vec![act("screen.show", "gear", t.get("gear.open"))],
    });
    let ready = g.save.unlocked.contains(&g.save.character);
    ui.panels.push(Panel {
        id: "launch".into(),
        kind: "bar".into(),
        actions: vec![
            back(g),
            hint(
                primary(enabled(act("run.start", "", t.get("hangar.launch")), ready)),
                t.get("key.enter"),
            ),
        ],
        ..Panel::default()
    });
}

/// Why something is locked, if it is: the achievement that opens it when
/// the grid shows that achievement, or a note that a hidden one does.
fn unlock_hint(g: &Game, kind: &str, id: &str) -> Option<String> {
    let t = &g.text;
    if !g.save.locked(&g.content, kind, id) {
        return None;
    }
    let a = g.content.unlocked_by(kind, id)?;
    Some(if g.save.visible(&g.content, &a.id) {
        t.fill("lock.by", &[("name", t.name(&a.id))])
    } else {
        t.get("lock.hidden")
    })
}

fn module_effect(g: &Game, id: &str, rank: i32) -> String {
    let per = g.content.module(id).map(|d| d.per_rank).unwrap_or(0);
    g.text
        .fill(&format!("module.{id}"), &[("n", (per * rank).to_string())])
}

/// What the chosen character's gear adds, in words.
fn gear_totals(g: &Game) -> String {
    let worn: Vec<String> = g
        .save
        .gear_of(&g.content, &g.save.character)
        .into_iter()
        .flatten()
        .map(|p| module_effect(g, &p.id, p.rank))
        .collect();
    if worn.is_empty() {
        g.text.get("gear.none_worn")
    } else {
        worn.join(" ")
    }
}

/// One item per gear slot of the chosen character.
fn doll_items(g: &Game, clickable: bool) -> Vec<Item> {
    let c = &g.content;
    let t = &g.text;
    let gear = g.save.gear_of(c, &g.save.character);
    let mut items = Vec::new();
    for (k, kind) in c.gear.slots.iter().enumerate() {
        let slot_name = t.get(&format!("gear.slot.{kind}"));
        let mut it = Item {
            id: format!("slot{k}"),
            text: slot_name.clone(),
            ..Item::default()
        };
        if !g.save.slot_open(c, k) {
            it.icon = format!("icons/slot_{kind}");
            it.title = t.get("gear.locked");
            it.tag = unlock_hint(g, "slot", &k.to_string()).unwrap_or_default();
            it.tone = "locked".into();
        } else if let Some(p) = gear[k] {
            let def = c.module(&p.id);
            it.icon = format!("icons/mod_{}", p.id);
            it.title = t.name(&p.id);
            it.tag = t.fill("gear.rank_short", &[("n", p.rank.to_string())]);
            it.tone = def.map(|d| rarity_tone(d.rarity)).unwrap_or("").into();
            it.selected = g.gear_pick == p.uid;
            if clickable {
                it.click = Some(act("gear.pick", &p.uid.to_string(), it.title.clone()));
            }
        } else {
            it.icon = format!("icons/slot_{kind}");
            it.title = t.get("gear.empty_slot");
            it.tone = "empty".into();
        }
        items.push(it);
    }
    items
}

fn gear(g: &Game, ui: &mut Ui) {
    let c = &g.content;
    let t = &g.text;
    let ch = &g.save.character;
    ui.panels.push(Panel {
        id: "who".into(),
        kind: "tabs".into(),
        heading: purse(g),
        actions: c
            .characters
            .iter()
            .filter(|x| g.save.unlocked.contains(&x.id))
            .map(|x| {
                let a = act("hangar.pick", &x.id, t.name(&x.id));
                if &x.id == ch {
                    primary(a)
                } else {
                    a
                }
            })
            .collect(),
        ..Panel::default()
    });
    ui.panels.push(Panel {
        id: "doll".into(),
        kind: "doll".into(),
        heading: t.fill("gear.heading", &[("name", t.name(ch))]),
        text: gear_totals(g),
        items: doll_items(g, true),
        ..Panel::default()
    });
    // the bag: by slot kind, then rarity high to low, then name and rank
    let mut pieces: Vec<&crate::meta::Piece> = g.save.modules.iter().collect();
    pieces.sort_by_key(|p| {
        let d = c.module(&p.id);
        (
            d.map(|d| c.gear.slots.iter().position(|s| *s == d.slot).unwrap_or(0))
                .unwrap_or(0),
            std::cmp::Reverse(d.map(|d| d.rarity)),
            p.id.clone(),
            std::cmp::Reverse(p.rank),
            p.uid,
        )
    });
    let worn_here = g.save.equipped.get(ch).cloned().unwrap_or_default();
    let items: Vec<Item> = pieces
        .iter()
        .map(|p| Item {
            id: format!("piece{}", p.uid),
            icon: format!("icons/mod_{}", p.id),
            title: t.name(&p.id),
            tag: t.fill("gear.rank_short", &[("n", p.rank.to_string())]),
            tone: c
                .module(&p.id)
                .map(|d| rarity_tone(d.rarity))
                .unwrap_or("")
                .into(),
            badge: if worn_here.contains(&p.uid) {
                t.get("gear.worn_badge")
            } else {
                String::new()
            },
            click: Some(act("gear.pick", &p.uid.to_string(), t.name(&p.id))),
            selected: g.gear_pick == p.uid,
            ..Item::default()
        })
        .collect();
    let can_merge = g.save.modules.iter().any(|p| {
        p.rank < c.cache.max_rank && g.save.merge_partners(c, p.uid).len() + 1 >= c.gear.merge_count
    });
    ui.panels.push(Panel {
        id: "inventory".into(),
        kind: "grid".into(),
        heading: t.fill("gear.inventory", &[("n", items.len().to_string())]),
        text: if items.is_empty() {
            t.get("gear.empty")
        } else {
            t.fill("gear.merge_rule", &[("n", c.gear.merge_count.to_string())])
        },
        items,
        actions: vec![enabled(
            act("gear.merge_all", "", t.get("gear.merge_all")),
            can_merge,
        )],
    });
    ui.panels.push(gear_detail(g));
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![
            back(g),
            act("screen.show", "cache", t.get("menu.cache")),
            primary(act("screen.show", "hangar", t.get("gear.to_hangar"))),
        ],
        ..Panel::default()
    });
}

fn gear_detail(g: &Game) -> Panel {
    let c = &g.content;
    let t = &g.text;
    let ch = &g.save.character;
    let mut panel = Panel {
        id: "detail".into(),
        kind: "detail".into(),
        ..Panel::default()
    };
    let Some(p) = g.save.piece(g.gear_pick) else {
        panel.text = t.get("gear.choose");
        return panel;
    };
    let Some(def) = c.module(&p.id) else { return panel };
    let worn_here = g.save.worn_by(p.uid).contains(ch);
    let others = g
        .save
        .modules
        .iter()
        .filter(|m| m.uid != p.uid && m.id == p.id && m.rank == p.rank)
        .count();
    let mut lines = vec![
        module_effect(g, &p.id, p.rank),
        t.fill(
            "gear.rank",
            &[("n", p.rank.to_string()), ("max", c.cache.max_rank.to_string())],
        ),
    ];
    if p.rank < c.cache.max_rank {
        lines.push(t.fill("gear.next", &[("effect", module_effect(g, &p.id, p.rank + 1))]));
        lines.push(t.fill(
            "gear.copies",
            &[
                ("n", others.to_string()),
                ("need", (c.gear.merge_count - 1).to_string()),
            ],
        ));
    }
    let worn: Vec<String> = g.save.worn_by(p.uid).iter().map(|x| t.name(x)).collect();
    if !worn.is_empty() {
        lines.push(t.fill("gear.worn_by", &[("names", worn.join(", "))]));
    }
    let base = match def.rarity {
        Rarity::Common => c.gear.scrap_common,
        Rarity::Rare => c.gear.scrap_rare,
        Rarity::Epic => c.gear.scrap_epic,
    };
    let scrap_gold = base << (p.rank - 1).clamp(0, 16);
    let uid = p.uid.to_string();
    let mut actions = vec![if worn_here {
        act("gear.unequip", &uid, t.get("gear.unequip"))
    } else {
        primary(act("gear.equip", &uid, t.get("gear.equip")))
    }];
    actions.push(enabled(
        act("gear.merge", &uid, t.get("gear.merge")),
        p.rank < c.cache.max_rank && others + 1 >= c.gear.merge_count,
    ));
    actions.push(enabled(
        act(
            "gear.scrap",
            &uid,
            t.fill("gear.scrap", &[("gold", scrap_gold.to_string())]),
        ),
        worn.is_empty(),
    ));
    panel.items.push(Item {
        id: "piece".into(),
        icon: format!("icons/mod_{}", p.id),
        title: t.name(&p.id),
        tag: format!(
            "{} · {}",
            t.get(&format!("rarity.{}", rarity_tone(def.rarity))),
            t.get(&format!("gear.slot.{}", def.slot))
        ),
        tone: rarity_tone(def.rarity).into(),
        text: lines.join(" "),
        actions,
        ..Item::default()
    });
    panel
}

fn rarity_tone(r: Rarity) -> &'static str {
    match r {
        Rarity::Common => "common",
        Rarity::Rare => "rare",
        Rarity::Epic => "epic",
    }
}

/// Where the next rank of an outfitter line comes from: the first unearned
/// achievement that opens one, named if the grid shows it.
fn next_rank_hint(g: &Game, id: &str) -> String {
    let t = &g.text;
    let next = g
        .content
        .achievements
        .iter()
        .filter(|a| a.unlocks() == Some(("rank", id)) && !g.save.claimed.contains(&a.id))
        .find(|a| g.save.visible(&g.content, &a.id));
    match next {
        Some(a) => t.fill("lock.by", &[("name", t.name(&a.id))]),
        None => t.get("lock.hidden"),
    }
}

fn shop(g: &Game, ui: &mut Ui) {
    let c = &g.content;
    let t = &g.text;
    let mut items = Vec::new();
    for s in &c.shop {
        let rank = g.save.ranks.get(&s.id).copied().unwrap_or(0);
        let cap = g.save.rank_cap(c, &s.id);
        let cost = crate::meta::Save::rank_cost(c, &s.id, rank);
        let action = match cost {
            Some(_) if rank >= cap => enabled(act("shop.buy", &s.id, t.get("shop.capped_label")), false),
            Some(cost) => enabled(
                act(
                    "shop.buy",
                    &s.id,
                    t.fill("shop.buy", &[("gold", cost.to_string())]),
                ),
                g.save.gold >= cost,
            ),
            None => enabled(act("shop.buy", &s.id, t.get("shop.maxed")), false),
        };
        items.push(Item {
            id: s.id.clone(),
            title: t.name(&s.id),
            text: if rank >= cap && rank < s.ranks {
                format!(
                    "{} {}",
                    t.fill(&format!("shop.about.{}", s.id), &[("n", s.per_rank.to_string())]),
                    next_rank_hint(g, &s.id)
                )
            } else {
                t.fill(&format!("shop.about.{}", s.id), &[("n", s.per_rank.to_string())])
            },
            tag: t.fill(
                "shop.rank",
                &[
                    ("n", rank.to_string()),
                    ("open", cap.to_string()),
                    ("max", s.ranks.to_string()),
                ],
            ),
            progress: Some(rank * 100 / s.ranks.max(1)),
            actions: vec![action],
            ..Item::default()
        });
    }
    ui.panels.push(Panel {
        id: "shop".into(),
        kind: "list".into(),
        heading: purse(g),
        text: t.get("shop.explain"),
        items,
        ..Panel::default()
    });
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![back(g)],
        ..Panel::default()
    });
}

fn cache(g: &Game, ui: &mut Ui) {
    let c = &g.content;
    let t = &g.text;
    let cc = &c.cache;
    let to_rare = cc.pity_rare.saturating_sub(g.save.pity_rare);
    ui.panels.push(Panel {
        id: "pull".into(),
        kind: "menu".into(),
        heading: purse(g),
        text: t.fill(
            "cache.explain",
            &[
                ("common", cc.common.to_string()),
                ("rare", cc.rare.to_string()),
                ("epic", cc.epic.to_string()),
                ("n", to_rare.to_string()),
                ("ten", cc.pity_rare.to_string()),
            ],
        ),
        actions: vec![
            enabled(
                act(
                    "cache.pull",
                    "1",
                    t.fill("cache.pull_one", &[("n", cc.pull_cost.to_string())]),
                ),
                g.save.tickets >= cc.pull_cost,
            ),
            primary(enabled(
                act(
                    "cache.pull",
                    "10",
                    t.fill("cache.pull_ten", &[("n", cc.ten_pull.to_string())]),
                ),
                g.save.tickets >= cc.ten_pull,
            )),
        ],
        ..Panel::default()
    });
    if !g.pulls.is_empty() {
        let items = g
            .pulls
            .iter()
            .enumerate()
            .map(|(k, p)| Item {
                id: format!("pull{k}"),
                icon: "icons/module".into(),
                title: t.name(&p.id),
                text: t.get(&format!("rarity.{}", rarity_tone(p.rarity))),
                tag: if p.new {
                    t.get("cache.new")
                } else {
                    t.fill(
                        "cache.rank",
                        &[("n", p.rank.to_string()), ("max", cc.max_rank.to_string())],
                    )
                },
                tone: rarity_tone(p.rarity).into(),
                ..Item::default()
            })
            .collect();
        let mut actions = Vec::new();
        if !g.reveal_all && !g.save.settings.reduced_motion {
            actions.push(act("cache.reveal", "", t.get("cache.skip")));
        }
        ui.panels.push(Panel {
            id: "reveal".into(),
            kind: if g.reveal_all || g.save.settings.reduced_motion {
                "reveal-done".into()
            } else {
                "reveal".into()
            },
            heading: t.get("cache.result"),
            items,
            actions,
            ..Panel::default()
        });
    }
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![back(g), act("screen.show", "gear", t.get("cache.to_hangar"))],
        ..Panel::default()
    });
}

fn reward_text(g: &Game, a: &crate::content::Achievement) -> String {
    let t = &g.text;
    let mut parts = vec![t.fill("ach.reward", &[("gold", a.gold.to_string())])];
    if a.tickets > 0 {
        parts.push(t.fill("ach.tickets", &[("n", a.tickets.to_string())]));
    }
    if let Some((kind, id)) = a.unlocks() {
        let name = match kind {
            "slot" => {
                let n: usize = id.parse().unwrap_or(0);
                let k = g.content.gear.slots.get(n).cloned().unwrap_or_default();
                t.get(&format!("gear.slot.{k}"))
            }
            "banish" => id.to_string(),
            _ => t.name(id),
        };
        parts.push(t.fill(&format!("ach.unlock.{kind}"), &[("name", name)]));
    }
    parts.join(" ")
}

fn ach_about(g: &Game, a: &crate::content::Achievement) -> String {
    let t = &g.text;
    let (now, target) = g.save.progress(&g.content, &a.id);
    let progress = if a.at_most {
        if now == 0 {
            t.get("ach.best_none")
        } else {
            t.fill("ach.best", &[("now", crate::text::clock(now as i32))])
        }
    } else {
        t.fill(
            "ach.progress",
            &[("now", now.to_string()), ("target", target.to_string())],
        )
    };
    let target_text = if a.at_most || a.stat.ends_with("_s") {
        crate::text::clock(a.target as i32)
    } else {
        a.target.to_string()
    };
    format!(
        "{} {}",
        t.fill(&format!("ach.{}", a.id), &[("n", target_text)]),
        progress
    )
}

fn achievements(g: &Game, ui: &mut Ui) {
    let c = &g.content;
    let t = &g.text;
    let mut items = Vec::new();
    for a in &c.achievements {
        let done = g.save.claimed.contains(&a.id);
        let seen = g.save.visible(c, &a.id);
        let (now, target) = g.save.progress(c, &a.id);
        let pct = if done {
            100
        } else if a.at_most {
            0
        } else {
            (now * 100 / target.max(1)) as i32
        };
        items.push(Item {
            id: a.id.clone(),
            icon: if seen {
                format!("icons/ach_{}", a.icon)
            } else {
                "icons/ach_hidden".into()
            },
            title: if seen {
                t.name(&a.id)
            } else {
                t.get("ach.hidden_title")
            },
            tone: if done {
                "done".into()
            } else if seen {
                String::new()
            } else {
                "hidden".into()
            },
            progress: if seen && !done { Some(pct) } else { None },
            at: Some(a.at),
            click: Some(act("ach.pick", &a.id, t.name(&a.id))),
            selected: g.ach_pick == a.id,
            ..Item::default()
        });
    }
    ui.panels.push(Panel {
        id: "achievements".into(),
        kind: "achgrid".into(),
        heading: t.fill(
            "ach.count",
            &[
                ("n", g.save.claimed.len().to_string()),
                ("max", c.achievements.len().to_string()),
            ],
        ),
        text: t.get("ach.explain"),
        items,
        ..Panel::default()
    });
    let mut detail = Panel {
        id: "detail".into(),
        kind: "detail".into(),
        ..Panel::default()
    };
    match c.achievement(&g.ach_pick) {
        None => detail.text = t.get("ach.choose"),
        Some(a) if !g.save.visible(c, &a.id) => {
            detail.items.push(Item {
                id: "ach".into(),
                icon: "icons/ach_hidden".into(),
                title: t.get("ach.hidden_title"),
                text: t.get("ach.hidden_about"),
                tone: "hidden".into(),
                ..Item::default()
            });
        }
        Some(a) => {
            let done = g.save.claimed.contains(&a.id);
            detail.items.push(Item {
                id: "ach".into(),
                icon: format!("icons/ach_{}", a.icon),
                title: t.name(&a.id),
                text: ach_about(g, a),
                tag: reward_text(g, a),
                tone: if done { "done".into() } else { String::new() },
                progress: if done || a.at_most {
                    None
                } else {
                    let (now, target) = g.save.progress(c, &a.id);
                    Some((now * 100 / target.max(1)) as i32)
                },
                ..Item::default()
            });
        }
    }
    ui.panels.push(detail);
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![back(g)],
        ..Panel::default()
    });
}

/// The codex tabs, in order.
pub const CODEX_TABS: &[&str] = &["abilities", "passives", "evolutions"];

fn level_line(g: &Game, id: &str, level: i32) -> String {
    let t = &g.text;
    let Some(d) = g.content.ability(id) else {
        return String::new();
    };
    let Some(l) = d.levels.get((level - 1).max(0) as usize) else {
        return String::new();
    };
    let mut line = t.fill(
        "codex.level",
        &[
            ("n", level.to_string()),
            ("damage", l.damage.to_string()),
            ("cd", seconds_text(l.cooldown_ms)),
        ],
    );
    if l.count > 1
        && matches!(
            d.kind,
            Kind::Shot | Kind::Bomb | Kind::Orbit | Kind::Chain | Kind::Turret
        )
    {
        line = format!("{} {}", line, t.fill("card.count", &[("n", l.count.to_string())]));
    }
    line
}

/// The ability whose evolution this is.
fn evolves_from<'a>(g: &'a Game, evo: &str) -> Option<&'a crate::content::Ability> {
    g.content.abilities.iter().find(|a| a.evolution == evo)
}

/// The recipe for an ability with no run to judge it by.
fn recipe_plain(g: &Game, id: &str) -> String {
    let t = &g.text;
    let c = &g.content;
    if c.is_evolution(id) {
        return match evolves_from(g, id) {
            Some(from) => t.fill(
                "recipe.from",
                &[
                    ("from", t.name(&from.id)),
                    ("partner", t.name(&from.evolves_with)),
                    ("max", c.run.max_level.to_string()),
                    ("plevel", c.partner_level(&from.evolves_with).to_string()),
                ],
            ),
            None => String::new(),
        };
    }
    let Some(d) = c.ability(id) else {
        return String::new();
    };
    if d.evolution.is_empty() {
        return t.get("recipe.none");
    }
    t.fill(
        "recipe.need",
        &[
            ("into", t.name(&d.evolution)),
            ("partner", t.name(&d.evolves_with)),
            ("max", c.run.max_level.to_string()),
            ("plevel", c.partner_level(&d.evolves_with).to_string()),
        ],
    )
}

pub fn codex(g: &Game) -> Panel {
    let c = &g.content;
    let t = &g.text;
    let mut items = Vec::new();
    match g.codex_tab.as_str() {
        "passives" => {
            for p in &c.passives {
                let mut text = format!(
                    "{} {}",
                    t.fill(&format!("about.{}", p.id), &[("n", p.per_level.to_string())]),
                    t.fill("codex.max_level", &[("n", c.passive_max(p).to_string())])
                );
                for a in c.abilities.iter().filter(|a| a.evolves_with == p.id) {
                    text = format!(
                        "{} {}",
                        text,
                        t.fill(
                            "recipe.partner_of",
                            &[
                                ("ability", t.name(&a.id)),
                                ("into", t.name(&a.evolution)),
                                ("plevel", c.partner_level(&p.id).to_string()),
                            ]
                        )
                    );
                }
                items.push(Item {
                    id: p.id.clone(),
                    icon: format!("icons/{}", p.id),
                    title: t.name(&p.id),
                    text,
                    tone: "rare".into(),
                    ..Item::default()
                });
            }
        }
        "evolutions" => {
            for e in &c.evolutions {
                let mut text = format!(
                    "{} {} {}",
                    t.get(&format!("about.{}", e.id)),
                    level_line(g, &e.id, 1),
                    recipe_plain(g, &e.id)
                );
                let lock = unlock_hint(g, "evolution", &e.id);
                if let Some(h) = &lock {
                    text = format!("{text} {h}");
                }
                items.push(Item {
                    id: e.id.clone(),
                    icon: format!("icons/{}", e.id),
                    title: t.name(&e.id),
                    text,
                    tag: if lock.is_some() {
                        t.get("codex.locked")
                    } else {
                        String::new()
                    },
                    tone: if lock.is_some() { "locked" } else { "epic" }.into(),
                    ..Item::default()
                });
            }
        }
        _ => {
            for a in &c.abilities {
                let mut text = format!(
                    "{} {} {} {}",
                    t.get(&format!("about.{}", a.id)),
                    level_line(g, &a.id, 1),
                    level_line(g, &a.id, c.run.max_level),
                    recipe_plain(g, &a.id)
                );
                let lock = unlock_hint(g, "ability", &a.id);
                if let Some(h) = &lock {
                    text = format!("{text} {h}");
                }
                let starter_of = c.characters.iter().find(|ch| ch.starter == a.id);
                let tag = if lock.is_some() {
                    t.get("codex.locked")
                } else if let Some(ch) = starter_of {
                    t.fill("codex.starter", &[("name", t.name(&ch.id))])
                } else if !a.owner.is_empty() {
                    t.fill("codex.only", &[("name", t.name(&a.owner))])
                } else {
                    String::new()
                };
                items.push(Item {
                    id: a.id.clone(),
                    icon: format!("icons/{}", a.id),
                    title: t.name(&a.id),
                    text,
                    tag,
                    tone: if lock.is_some() {
                        "locked".into()
                    } else {
                        String::new()
                    },
                    ..Item::default()
                });
            }
        }
    }
    let mut actions: Vec<Action> = CODEX_TABS
        .iter()
        .map(|tab| {
            let a = act("codex.tab", tab, t.get(&format!("codex.tab.{tab}")));
            if g.codex_tab == *tab {
                primary(a)
            } else {
                a
            }
        })
        .collect();
    if g.world.is_some() {
        actions.push(act("codex.close", "", t.get("codex.back")));
    }
    Panel {
        id: "codex".into(),
        kind: "codex".into(),
        heading: t.get("codex.heading"),
        text: t.get("codex.explain"),
        items,
        actions,
    }
}

fn toggle(g: &Game, name: &str, key: &str, on: bool) -> Item {
    let t = &g.text;
    Item {
        id: key.into(),
        title: t.get(&format!("settings.{key}")),
        text: t.get(&format!("settings.{key}.about")),
        tag: t.get(if on { "settings.on" } else { "settings.off" }),
        tone: if on { "done".into() } else { String::new() },
        actions: vec![act(
            name,
            "",
            t.get(if on {
                "settings.turn_off"
            } else {
                "settings.turn_on"
            }),
        )],
        ..Item::default()
    }
}

fn settings(g: &Game, ui: &mut Ui) {
    let s = &g.save.settings;
    ui.panels.push(Panel {
        id: "settings".into(),
        kind: "list".into(),
        items: vec![
            toggle(g, "settings.autoaim", "autoaim", s.autoaim),
            toggle(g, "settings.motion", "motion", s.reduced_motion),
        ],
        ..Panel::default()
    });
    ui.panels.push(save_panel(g));
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![back(g)],
        ..Panel::default()
    });
}

fn help(g: &Game, ui: &mut Ui) {
    let t = &g.text;
    ui.panels.push(Panel {
        id: "help".into(),
        kind: "prose".into(),
        heading: t.get("help.heading"),
        text: t.get("help.text"),
        items: ["move", "aim", "dash", "cards", "pause"]
            .iter()
            .map(|k| Item {
                id: (*k).into(),
                title: t.get(&format!("help.{k}")),
                text: t.get(&format!("help.{k}.about")),
                ..Item::default()
            })
            .collect(),
        ..Panel::default()
    });
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![back(g)],
        ..Panel::default()
    });
}

fn coop(g: &Game, ui: &mut Ui) {
    let t = &g.text;
    let status = g.session.as_ref().map(|n| n.status).unwrap_or("");
    let connected = matches!(status, online::LOBBY | online::PLAYING);
    let mut text = String::new();
    if !g.net_error.is_empty() {
        text = t.get(&g.net_error);
    } else if !status.is_empty() {
        text = t.get(status);
    }
    let mut actions = Vec::new();
    match &g.session {
        None => {
            actions.push(primary(act("page.host", "", t.get("coop.host"))));
            actions.push(act("page.join", "", t.get("coop.join")));
            actions.push(act("page.invite", "", t.get("coop.invite")));
            actions.push(act("page.reply", "", t.get("coop.reply")));
            text = t.get("coop.explain");
        }
        Some(n) => {
            if n.seat == 0 {
                let partner = g.peer_load.as_ref().map(|l| t.name(&l.character));
                actions.push(primary(enabled(
                    act("coop.start", "", t.get("coop.start")),
                    connected && partner.is_some(),
                )));
                if let Some(p) = partner {
                    text = format!("{} {}", text, t.fill("coop.partner", &[("name", p)]));
                }
            } else if connected {
                text = format!("{} {}", text, t.get("coop.wait_host"));
            }
            actions.push(act("coop.leave", "", t.get("coop.leave")));
        }
    }
    let heading = if g.room.is_empty() {
        t.get("coop.heading")
    } else {
        t.fill("coop.room", &[("code", g.room.clone())])
    };
    ui.panels.push(Panel {
        id: "coop".into(),
        kind: "menu".into(),
        heading,
        text,
        actions,
        ..Panel::default()
    });
    let items = g
        .content
        .characters
        .iter()
        .filter(|c| g.save.unlocked.contains(&c.id))
        .map(|c| character_item(g, &c.id))
        .collect();
    ui.panels.push(Panel {
        id: "characters".into(),
        kind: "cards".into(),
        heading: t.get("coop.your_character"),
        items,
        ..Panel::default()
    });
    if g.session.is_none() {
        ui.panels.push(Panel {
            id: "nav".into(),
            kind: "bar".into(),
            actions: vec![back(g)],
            ..Panel::default()
        });
    }
}

fn card_item(g: &Game, seat: usize, k: usize, card: &crate::world::Card, banish: bool) -> Item {
    let c = &g.content;
    let t = &g.text;
    let w = g.world.as_ref().expect("a run");
    let n = (k + 1).to_string();
    let mut actions = vec![hint(primary(act("card.take", &n, t.get("card.take"))), n.clone())];
    if banish && !card.id.is_empty() {
        actions.push(act("card.banish", &n, t.get("card.banish")));
    }
    let (title, text, tag, icon, tone) = match card.kind {
        CardKind::NewAbility | CardKind::AbilityUp | CardKind::Evolve => {
            let def = c.ability(&card.id);
            let mut body = t.get(&format!("about.{}", card.id));
            if let Some(s) = w.scaled(c, seat, &card.id, card.level) {
                let lv = def.and_then(|d| d.levels.get((card.level - 1).max(0) as usize));
                let cd_ms = lv.map(|l| l.cooldown_ms).unwrap_or(0);
                let mut stats = t.fill(
                    "card.stats",
                    &[("damage", s.damage.to_string()), ("cd", seconds_text(cd_ms))],
                );
                if s.count > 1
                    && matches!(
                        s.kind,
                        Kind::Shot | Kind::Bomb | Kind::Orbit | Kind::Chain | Kind::Turret
                    )
                {
                    stats = format!(
                        "{} {}",
                        stats,
                        t.fill("card.count", &[("n", s.count.to_string())])
                    );
                }
                body = format!("{body} {stats}");
            }
            if let Some(d) = def {
                if !d.evolves_with.is_empty() {
                    body = format!(
                        "{} {}",
                        body,
                        t.fill(
                            "recipe.need",
                            &[
                                ("partner", t.name(&d.evolves_with)),
                                ("into", t.name(&d.evolution)),
                                ("max", c.run.max_level.to_string()),
                                ("plevel", c.partner_level(&d.evolves_with).to_string())
                            ]
                        )
                    );
                }
            }
            let tag = match card.kind {
                CardKind::NewAbility => t.get("card.new"),
                CardKind::Evolve => t.get("card.evolve"),
                _ => t.fill("card.level", &[("n", card.level.to_string())]),
            };
            let tone = if card.kind == CardKind::Evolve { "epic" } else { "" };
            (t.name(&card.id), body, tag, format!("icons/{}", card.id), tone)
        }
        CardKind::NewPassive | CardKind::PassiveUp => {
            let per = c.passive(&card.id).map(|p| p.per_level).unwrap_or(0);
            let mut body = t.fill(&format!("about.{}", card.id), &[("n", per.to_string())]);
            let partners: Vec<String> = c
                .abilities
                .iter()
                .filter(|a| a.evolves_with == card.id)
                .map(|a| t.name(&a.id))
                .collect();
            if !partners.is_empty() {
                body = format!(
                    "{} {}",
                    body,
                    t.fill("card.partner_of", &[("names", partners.join(", "))])
                );
            }
            let tag = if card.kind == CardKind::NewPassive {
                t.get("card.new")
            } else {
                t.fill("card.level", &[("n", card.level.to_string())])
            };
            (t.name(&card.id), body, tag, format!("icons/{}", card.id), "rare")
        }
        CardKind::Gold => (
            t.get("card.gold"),
            t.get("card.gold.about"),
            String::new(),
            "icons/gold".into(),
            "",
        ),
        CardKind::Heal => (
            t.get("card.heal"),
            t.fill("card.heal.about", &[("n", card.level.to_string())]),
            String::new(),
            "fx/health_cell".into(),
            "",
        ),
        CardKind::Salvage => (
            t.get("card.salvage"),
            t.fill("card.salvage.about", &[("n", card.level.to_string())]),
            String::new(),
            "icons/gold".into(),
            "",
        ),
    };
    Item {
        id: format!("card{k}"),
        icon,
        title,
        text,
        tag,
        tone: tone.into(),
        actions,
        ..Item::default()
    }
}

/// Where an ability held in a run stands on its way to evolving (T2).
fn recipe_status(g: &Game, w: &crate::world::World, seat: usize, k: usize) -> String {
    let t = &g.text;
    let c = &g.content;
    let p = &w.players[seat];
    let slot = &p.abilities[k];
    if c.is_evolution(&slot.id) {
        return recipe_plain(g, &slot.id);
    }
    let Some(d) = c.ability(&slot.id) else {
        return String::new();
    };
    if d.evolution.is_empty() {
        return t.get("recipe.none");
    }
    if p.load.locked.contains(&d.evolution) {
        return format!(
            "{} {}",
            recipe_plain(g, &slot.id),
            unlock_hint(g, "evolution", &d.evolution).unwrap_or_else(|| t.get("lock.hidden"))
        );
    }
    if w.evolvable(c, seat) == Some(k) {
        return t.fill("recipe.ready", &[("into", t.name(&d.evolution))]);
    }
    let held = p
        .passives
        .iter()
        .find(|q| q.id == d.evolves_with)
        .map(|q| q.level)
        .unwrap_or(0);
    format!(
        "{} {} {}",
        recipe_plain(g, &slot.id),
        t.fill(
            "recipe.level_now",
            &[
                ("n", slot.level.to_string()),
                ("max", c.run.max_level.to_string()),
                ("plevel", c.run.evolve_partner_level.to_string())
            ]
        ),
        if held > 0 {
            t.fill(
                "recipe.holding",
                &[("partner", t.name(&d.evolves_with)), ("n", held.to_string())],
            )
        } else {
            t.fill("recipe.missing", &[("partner", t.name(&d.evolves_with))])
        }
    )
}

/// The build, item by item, with what each does now and how it evolves.
fn build_details(g: &Game, w: &crate::world::World, seat: usize) -> Vec<Item> {
    let t = &g.text;
    let c = &g.content;
    let p = &w.players[seat];
    let ready = w.evolvable(c, seat);
    let mut items = Vec::new();
    for (k, slot) in p.abilities.iter().enumerate() {
        let mut text = t.get(&format!("about.{}", slot.id));
        if let Some(s) = w.scaled(c, seat, &slot.id, slot.level) {
            let cd = c
                .ability(&slot.id)
                .and_then(|d| d.levels.get((slot.level - 1).max(0) as usize))
                .map(|l| l.cooldown_ms)
                .unwrap_or(0);
            text = format!(
                "{} {}",
                text,
                t.fill(
                    "card.stats",
                    &[("damage", s.damage.to_string()), ("cd", seconds_text(cd))]
                )
            );
        }
        text = format!("{} {}", text, recipe_status(g, w, seat, k));
        items.push(Item {
            id: slot.id.clone(),
            icon: format!("icons/{}", slot.id),
            title: t.name(&slot.id),
            tag: if c.is_evolution(&slot.id) {
                t.get("build.evolved")
            } else {
                t.fill("card.level", &[("n", slot.level.to_string())])
            },
            tone: if ready == Some(k) {
                "epic".into()
            } else {
                String::new()
            },
            text,
            ..Item::default()
        });
    }
    for slot in &p.passives {
        let per = c.passive(&slot.id).map(|d| d.per_level).unwrap_or(0);
        let mut text = t.fill(&format!("about.{}", slot.id), &[("n", per.to_string())]);
        for a in c.abilities.iter().filter(|a| a.evolves_with == slot.id) {
            text = format!(
                "{} {}",
                text,
                t.fill(
                    "recipe.partner_of",
                    &[
                        ("ability", t.name(&a.id)),
                        ("into", t.name(&a.evolution)),
                        ("plevel", c.partner_level(&slot.id).to_string()),
                    ]
                )
            );
        }
        items.push(Item {
            id: slot.id.clone(),
            icon: format!("icons/{}", slot.id),
            title: t.name(&slot.id),
            tag: t.fill("card.level", &[("n", slot.level.to_string())]),
            tone: "rare".into(),
            text,
            ..Item::default()
        });
    }
    items
}

fn run(g: &Game, ui: &mut Ui) {
    let t = &g.text;
    let Some(w) = g.world.as_ref() else { return };
    let c = &g.content;
    let me = &w.players[g.seat.min(w.players.len() - 1)];
    if let Some(offer) = &me.offer {
        let banish = offer.kind == OfferKind::LevelUp && me.banishes > 0;
        let items = offer
            .cards
            .iter()
            .enumerate()
            .map(|(k, card)| card_item(g, me.seat, k, card, banish))
            .collect();
        let mut actions = Vec::new();
        if offer.kind == OfferKind::LevelUp {
            actions.push(hint(
                enabled(
                    act(
                        "card.reroll",
                        "",
                        t.fill("card.reroll", &[("n", me.rerolls.to_string())]),
                    ),
                    me.rerolls > 0,
                ),
                t.get("key.reroll"),
            ));
        }
        ui.overlay = Some(Panel {
            id: "offer".into(),
            kind: "cards".into(),
            heading: t.get(if offer.kind == OfferKind::LevelUp {
                "card.heading"
            } else {
                "card.reward"
            }),
            text: if offer.kind == OfferKind::LevelUp {
                t.fill("card.banishes", &[("n", me.banishes.to_string())])
            } else {
                String::new()
            },
            items,
            actions,
        });
    } else if w.waiting() {
        ui.overlay = Some(Panel {
            id: "waiting".into(),
            kind: "note".into(),
            heading: t.get("coop.partner_choosing"),
            ..Panel::default()
        });
    } else if g.overlay_open() && g.codex_open {
        ui.overlay = Some(codex(g));
    } else if g.overlay_open() {
        let mut actions = vec![hint(
            primary(act(
                "run.resume",
                "",
                t.get(if g.paused { "pause.resume" } else { "inspect.close" }),
            )),
            t.get(if g.paused { "key.pause" } else { "key.inspect" }),
        )];
        actions.push(act("codex.open", "", t.get("inspect.codex")));
        if g.paused {
            actions.push(act("run.quit", "", t.get("pause.quit")));
        }
        ui.overlay = Some(Panel {
            id: "pause".into(),
            kind: "build".into(),
            heading: t.get(if g.paused {
                "pause.heading"
            } else {
                "inspect.heading"
            }),
            text: t.get("inspect.explain"),
            items: build_details(g, w, me.seat),
            actions,
        });
    } else if !me.alive {
        ui.overlay = Some(Panel {
            id: "down".into(),
            kind: "note".into(),
            heading: t.get("down.heading"),
            text: t.get("down.text"),
            ..Panel::default()
        });
    }
    let ready = w.evolvable(c, me.seat);
    for (k, slot) in me.abilities.iter().enumerate() {
        let is_ready = ready == Some(k);
        ui.build.push(Item {
            id: slot.id.clone(),
            icon: format!("icons/{}", slot.id),
            title: t.name(&slot.id),
            tag: if c.is_evolution(&slot.id) {
                t.get("build.evolved")
            } else if is_ready {
                t.get("build.ready")
            } else {
                t.fill("card.level", &[("n", slot.level.to_string())])
            },
            tone: if is_ready { "epic".into() } else { String::new() },
            ..Item::default()
        });
    }
    for slot in &me.passives {
        ui.build.push(Item {
            id: slot.id.clone(),
            icon: format!("icons/{}", slot.id),
            title: t.name(&slot.id),
            tag: t.fill("card.level", &[("n", slot.level.to_string())]),
            tone: "rare".into(),
            ..Item::default()
        });
    }
    ui.panels.push(Panel {
        id: "runbar".into(),
        kind: "bar".into(),
        actions: if g.coop_running() {
            vec![
                hint(
                    act("run.inspect", "", t.get("inspect.button")),
                    t.get("key.inspect"),
                ),
                act("run.quit", "", t.get("coop.leave")),
            ]
        } else {
            vec![hint(
                act("run.pause", "", t.get("pause.button")),
                t.get("key.pause"),
            )]
        },
        ..Panel::default()
    });
}

fn results(g: &Game, ui: &mut Ui) {
    let t = &g.text;
    let Some(p) = &g.payout else { return };
    ui.title = t.get(if p.won { "results.won" } else { "results.lost" });
    let mut items = vec![
        Item {
            id: "time".into(),
            title: t.get("results.time"),
            tag: crate::text::clock(p.seconds),
            ..Item::default()
        },
        Item {
            id: "kills".into(),
            title: t.get("results.kills"),
            tag: p.kills.to_string(),
            ..Item::default()
        },
        Item {
            id: "level".into(),
            title: t.get("results.level"),
            tag: p.level.to_string(),
            ..Item::default()
        },
        Item {
            id: "gold".into(),
            icon: "icons/gold".into(),
            title: t.get("results.gold"),
            tag: p.gold.to_string(),
            ..Item::default()
        },
        Item {
            id: "tickets".into(),
            icon: "icons/ticket".into(),
            title: t.get("results.tickets"),
            tag: p.tickets.to_string(),
            ..Item::default()
        },
    ];
    for a in &p.achievements {
        items.push(Item {
            id: a.clone(),
            title: t.fill("results.achievement", &[("name", t.name(a))]),
            tone: "done".into(),
            ..Item::default()
        });
    }
    ui.panels.push(Panel {
        id: "results".into(),
        kind: "list".into(),
        heading: purse(g),
        text: if g.saved_since_run {
            String::new()
        } else {
            t.get("results.save_reminder")
        },
        items,
        ..Panel::default()
    });
    let mut actions = vec![primary(act("page.download", "", t.get("save.download")))];
    if g.session.as_ref().map(|n| n.peer.is_some()).unwrap_or(false) {
        actions.push(act("screen.show", "coop", t.get("results.lobby")));
    } else {
        actions.push(act("run.start", "", t.get("results.again")));
    }
    actions.push(act("screen.show", "hangar", t.get("results.hangar")));
    actions.push(back(g));
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions,
        ..Panel::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tests::game;
    use crate::game::Raw;

    #[test]
    fn the_build_says_when_an_ability_is_ready_to_evolve() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        let max = g.content.run.max_level;
        let w = g.world.as_mut().unwrap();
        w.players[0].abilities[0].level = max;
        w.players[0].passives.push(crate::world::Slot {
            id: "split_feed".into(),
            level: 1,
            cd: 0,
        });
        g.act("run.pause", "").unwrap();
        let ui = build(&g);
        let rifle = &ui.overlay.as_ref().unwrap().items[0];
        let need = g.text.fill(
            "recipe.holding",
            &[("partner", g.text.name("split_feed")), ("n", "1".into())],
        );
        assert!(rifle.text.contains(&need), "{}", rifle.text);
        assert_eq!(ui.build[0].tag, g.text.get("card.level").replace("{n}", "5"));
        // split_feed tops out at level 2, which is its partner level
        g.world.as_mut().unwrap().players[0].passives[0].level = 2;
        let ui = build(&g);
        let rifle = &ui.overlay.as_ref().unwrap().items[0];
        let ready = g
            .text
            .fill("recipe.ready", &[("into", g.text.name("swarm_battery"))]);
        assert!(rifle.text.contains(&ready), "{}", rifle.text);
        assert_eq!(rifle.tone, "epic");
        assert_eq!(ui.build[0].tag, g.text.get("build.ready"));
    }

    #[test]
    fn hidden_achievements_show_as_question_marks_and_name_nothing() {
        let mut g = game();
        g.act("screen.show", "achievements").unwrap();
        let ui = build(&g);
        let grid = &ui.panels[0];
        let tile = |id: &str| grid.items.iter().find(|i| i.id == id).unwrap().clone();
        assert_eq!(tile("first_steps").title, g.text.name("first_steps"));
        let hidden = tile("marathon");
        assert_eq!(hidden.title, g.text.get("ach.hidden_title"));
        assert_eq!(hidden.icon, "icons/ach_hidden");
        assert_eq!(hidden.at, Some([0, 0]));
        g.act("ach.pick", "marathon").unwrap();
        let ui = build(&g);
        let detail = &ui.panels[1].items[0];
        assert!(!detail.text.contains(&g.text.name("marathon")));
        assert_eq!(detail.text, g.text.get("ach.hidden_about"));
    }

    /// Every screen, every card kind and the results render with no string
    /// missing from data/strings.json.
    #[test]
    fn every_screen_finds_all_its_strings() {
        let mut g = game();
        g.save.gold = 5000;
        g.save.tickets = 30;
        for s in [
            "title",
            "hangar",
            "shop",
            "cache",
            "achievements",
            "settings",
            "coop",
            "help",
            "gear",
            "codex",
        ] {
            g.act("screen.show", s).unwrap();
            build(&g);
        }
        for tab in CODEX_TABS {
            g.act("codex.tab", tab).unwrap();
            build(&g);
        }
        g.act("screen.show", "achievements").unwrap();
        g.act("ach.pick", "first_steps").unwrap();
        build(&g);
        g.act("ach.pick", "marathon").unwrap();
        build(&g);
        // every achievement's goal and reward, seen
        g.save.claimed = g.content.achievements.iter().map(|a| a.id.clone()).collect();
        for a in g.content.achievements.clone() {
            g.act("ach.pick", &a.id).unwrap();
            build(&g);
        }
        g.save.claimed.clear();
        g.act("screen.show", "cache").unwrap();
        g.act("cache.pull", "10").unwrap();
        build(&g);
        g.act("hangar.buy", "breaker").unwrap();
        g.save.claimed.push("burrower_down".into());
        g.save.unlocked.push("vector".into());
        g.save.tickets = 60;
        g.act("screen.show", "cache").unwrap();
        for _ in 0..6 {
            g.act("cache.pull", "10").unwrap();
        }
        g.act("screen.show", "gear").unwrap();
        for p in g.save.modules.clone() {
            g.act("gear.pick", &p.uid.to_string()).unwrap();
            build(&g);
        }
        let first = g.save.modules[0].uid;
        g.act("gear.pick", &first.to_string()).unwrap();
        g.act("gear.equip", &first.to_string()).unwrap();
        build(&g);
        g.act("screen.show", "hangar").unwrap();
        build(&g);
        for ch in ["pathfinder", "breaker", "vector"] {
            g.act("screen.show", "hangar").unwrap();
            g.act("hangar.pick", ch).unwrap();
            g.act("run.start", "").unwrap();
            for f in 0..(60 * 40) {
                g.advance(f * 16_667, Raw::default());
                build(&g);
                if g.world
                    .as_ref()
                    .map(|w| w.players[0].offer.is_some())
                    .unwrap_or(false)
                {
                    g.act("card.take", "1").unwrap();
                }
            }
            // every card the content can offer
            let content = g.content.clone();
            if let Some(w) = g.world.as_mut() {
                let mut cards = w.card_pool(&content, 0);
                for kind in [CardKind::Gold, CardKind::Heal, CardKind::Salvage] {
                    cards.push(crate::world::Card {
                        kind,
                        id: String::new(),
                        level: 1,
                    });
                }
                for e in &content.evolutions {
                    cards.push(crate::world::Card {
                        kind: CardKind::Evolve,
                        id: e.id.clone(),
                        level: 1,
                    });
                }
                for a in &content.abilities {
                    cards.push(crate::world::Card {
                        kind: CardKind::NewAbility,
                        id: a.id.clone(),
                        level: 1,
                    });
                }
                for p in &content.passives {
                    cards.push(crate::world::Card {
                        kind: CardKind::NewPassive,
                        id: p.id.clone(),
                        level: 1,
                    });
                }
                w.players[0].offer = Some(crate::world::Offer {
                    kind: OfferKind::LevelUp,
                    cards,
                });
            }
            build(&g);
            if let Some(w) = g.world.as_mut() {
                w.players[0].offer = None;
            }
            g.act("run.pause", "").unwrap();
            build(&g);
            g.act("codex.open", "").unwrap();
            build(&g);
            g.act("codex.close", "").unwrap();
            g.act("run.quit", "").unwrap();
            build(&g);
        }
        for e in &g.content.enemies {
            g.text.name(&e.id);
        }
        for key in [
            "banner.boss",
            "banner.raid_warning",
            "banner.raid_boss",
            "banner.raid_over",
            "banner.rift_open",
            "banner.rift_enter",
            "banner.rift_won",
            "banner.rift_lost",
            "banner.final",
            "banner.evolved",
            "banner.revived",
            "hud.dash_ready",
            "hud.dash_wait",
            "hud.dash_prompt",
            "inspect.heading",
            "inspect.close",
            "inspect.button",
            "key.inspect",
            "hud.final",
            "hud.seat",
            "hud.level",
            "hud.kills",
            "hud.rift",
        ] {
            g.text.get(key);
        }
        for key in [
            "save.not_json",
            "save.wrong_game",
            "save.wrong_version",
            "save.damaged",
            "save.done",
            "save.loaded",
            "shop.maxed",
            "shop.short",
            "shop.unknown",
            "cache.short",
            "shop.capped",
            "gear.unknown",
            "gear.wrong_slot",
            "gear.slot_locked",
            "gear.no_pair",
            "gear.max_rank",
            "gear.worn",
            "hangar.locked",
            "run.busy",
            "coop.no_partner",
            "online.waiting",
            "online.lobby",
            "online.playing",
            "online.reload",
            "online.garbled",
            "online.left",
            "online.full",
            "online.desync",
            "online.no_bundle",
            "online.no_relay",
            "online.no_path",
            "online.signal_failed",
            "online.bad_code",
            "coop.code_prompt",
            "coop.invite_prompt",
            "coop.reply_prompt",
            "coop.copied",
            "page.no_file",
            "page.loading",
            "screen.run.title",
            "screen.results.title",
        ] {
            g.text.get(key);
        }
        let missing = g.text.missing.borrow().clone();
        assert!(missing.is_empty(), "strings.json lacks: {missing:?}");
    }
}
