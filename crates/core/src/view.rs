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
            act("screen.show", "shop", t.get("menu.shop")),
            act("screen.show", "cache", t.get("menu.cache")),
            act("screen.show", "achievements", t.get("menu.achievements")),
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
        tag = t.fill("hangar.unlock_by", &[("name", t.name(&ch.unlock_achievement))]);
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
    ui.panels.push(modules_panel(g));
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

fn modules_panel(g: &Game) -> Panel {
    let c = &g.content;
    let t = &g.text;
    let ch = &g.save.character;
    let on: Vec<String> = g.save.equipped.get(ch).cloned().unwrap_or_default();
    let mut items = Vec::new();
    for m in &g.save.modules {
        let Some(def) = c.module(&m.id) else { continue };
        let equipped = on.contains(&m.id);
        items.push(Item {
            id: m.id.clone(),
            icon: "icons/module".into(),
            title: t.name(&m.id),
            text: t.fill(
                &format!("module.{}", m.id),
                &[("n", (def.per_rank * m.rank).to_string())],
            ),
            tag: t.fill(
                "cache.rank",
                &[("n", m.rank.to_string()), ("max", c.cache.max_rank.to_string())],
            ),
            tone: rarity_tone(def.rarity).into(),
            actions: vec![act(
                "module.toggle",
                &m.id,
                t.get(if equipped {
                    "hangar.unequip"
                } else {
                    "hangar.equip"
                }),
            )],
            ..Item::default()
        });
    }
    Panel {
        id: "modules".into(),
        kind: "list".into(),
        heading: t.fill(
            "hangar.modules",
            &[
                ("name", t.name(ch)),
                ("n", on.len().to_string()),
                ("max", c.cache.slots.to_string()),
            ],
        ),
        text: if items.is_empty() {
            t.get("hangar.no_modules")
        } else {
            String::new()
        },
        items,
        ..Panel::default()
    }
}

fn rarity_tone(r: Rarity) -> &'static str {
    match r {
        Rarity::Common => "common",
        Rarity::Rare => "rare",
        Rarity::Epic => "epic",
    }
}

fn shop(g: &Game, ui: &mut Ui) {
    let c = &g.content;
    let t = &g.text;
    let mut items = Vec::new();
    for s in &c.shop {
        let rank = g.save.ranks.get(&s.id).copied().unwrap_or(0);
        let cost = crate::meta::Save::rank_cost(c, &s.id, rank);
        let action = match cost {
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
            text: t.fill(&format!("shop.about.{}", s.id), &[("n", s.per_rank.to_string())]),
            tag: t.fill(
                "shop.rank",
                &[("n", rank.to_string()), ("max", s.ranks.to_string())],
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
        actions: vec![back(g), act("screen.show", "hangar", t.get("cache.to_hangar"))],
        ..Panel::default()
    });
}

fn achievements(g: &Game, ui: &mut Ui) {
    let c = &g.content;
    let t = &g.text;
    let mut items = Vec::new();
    for a in &c.achievements {
        let (now, target) = g.save.progress(c, &a.id);
        let done = g.save.claimed.contains(&a.id);
        let mut reward = t.fill(
            "ach.reward",
            &[
                ("gold", a.gold.to_string()),
                ("xp", c.achievement_xp_pct.to_string()),
            ],
        );
        if a.tickets > 0 {
            reward = format!(
                "{} {}",
                reward,
                t.fill("ach.tickets", &[("n", a.tickets.to_string())])
            );
        }
        if t.has(&format!("ach.unlock.{}", a.id)) {
            reward = format!("{} {}", reward, t.get(&format!("ach.unlock.{}", a.id)));
        }
        items.push(Item {
            id: a.id.clone(),
            title: t.name(&a.id),
            text: format!(
                "{} {}",
                t.fill(&format!("ach.{}", a.id), &[("n", a.target.to_string())]),
                t.fill(
                    "ach.progress",
                    &[("now", now.to_string()), ("target", target.to_string())]
                )
            ),
            tag: reward,
            tone: if done { "done".into() } else { String::new() },
            progress: Some((now * 100 / target.max(1)) as i32),
            ..Item::default()
        });
    }
    ui.panels.push(Panel {
        id: "achievements".into(),
        kind: "list".into(),
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
    ui.panels.push(Panel {
        id: "nav".into(),
        kind: "bar".into(),
        actions: vec![back(g)],
        ..Panel::default()
    });
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
                            "card.evolves",
                            &[
                                ("partner", t.name(&d.evolves_with)),
                                ("into", t.name(&d.evolution))
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
    } else if g.paused {
        ui.overlay = Some(Panel {
            id: "pause".into(),
            kind: "menu".into(),
            heading: t.get("pause.heading"),
            actions: vec![
                primary(act("run.resume", "", t.get("pause.resume"))),
                act("run.quit", "", t.get("pause.quit")),
            ],
            ..Panel::default()
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
    for slot in &me.abilities {
        ui.build.push(Item {
            id: slot.id.clone(),
            icon: format!("icons/{}", slot.id),
            title: t.name(&slot.id),
            tag: if c.is_evolution(&slot.id) {
                t.get("build.evolved")
            } else {
                t.fill("card.level", &[("n", slot.level.to_string())])
            },
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
            vec![act("run.quit", "", t.get("coop.leave"))]
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
        ] {
            g.act("screen.show", s).unwrap();
            build(&g);
        }
        g.act("screen.show", "cache").unwrap();
        g.act("cache.pull", "10").unwrap();
        build(&g);
        g.act("hangar.buy", "breaker").unwrap();
        g.save.claimed.push("burrower_down".into());
        g.save.unlocked.push("vector".into());
        for m in g.content.modules.clone() {
            g.save.modules.push(crate::meta::Owned {
                id: m.id.clone(),
                rank: 1,
            });
        }
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
            g.act("run.pause", "").unwrap();
            build(&g);
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
            "cache.unknown",
            "cache.full",
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
