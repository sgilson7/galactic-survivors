//! The run as a draw list, from the local player's camera. Order and
//! weight follow the readability rules (I7): a calm ground, then pickups and
//! obstacles, then the player's own effects drawn faint, then enemies, then
//! players, and enemy shots last and strongest so a threat is never under
//! anything. There is no screen shake.

use crate::content::Content;
use crate::draw::{sprite, Align, Color, DrawList};
use crate::fx;
use crate::game::Game;
use crate::geom::{SUB, V};
use crate::text::clock;
use crate::world::{EffectKind, PickupKind, World};

/// Thousandths of a screen pixel per world pixel, from the view size: about
/// 1280 by 720 world pixels are visible, never fewer than 700 across.
pub fn zoom(view_w: i32, view_h: i32) -> i32 {
    let by_w = view_w * 1000 / 1280;
    let by_h = view_h * 1000 / 720;
    by_w.min(by_h).clamp(450, 1400)
}

struct Cam {
    cx: i32,
    cy: i32,
    w: i32,
    h: i32,
    z: i32,
}

impl Cam {
    /// A world position (sub-pixels) on screen.
    fn at(&self, p: V) -> (i32, i32) {
        let x = (p.x - self.cx) as i64 * self.z as i64 / (SUB as i64 * 1000);
        let y = (p.y - self.cy) as i64 * self.z as i64 / (SUB as i64 * 1000);
        (self.w / 2 + x as i32, self.h / 2 + y as i32)
    }

    fn len(&self, px: i32) -> i32 {
        (px * self.z / 1000).max(1)
    }

    fn on(&self, (x, y): (i32, i32), margin: i32) -> bool {
        x > -margin && y > -margin && x < self.w + margin && y < self.h + margin
    }
}

fn seat_color(seat: usize) -> Color {
    if seat == 0 {
        Color::Seat0
    } else {
        Color::Seat1
    }
}

pub fn frame(g: &Game) -> Vec<i32> {
    let mut d = DrawList::new();
    let Some(w) = g.world.as_ref() else {
        d.clear(Color::Ground);
        return d.finish();
    };
    let c = &g.content;
    let me = &w.players[g.seat.min(w.players.len() - 1)];
    let cam = Cam {
        cx: me.pos.x,
        cy: me.pos.y,
        w: g.view_w,
        h: g.view_h,
        z: zoom(g.view_w, g.view_h),
    };
    let calm = g.save.settings.reduced_motion;
    d.clear(Color::Ground);
    if w.in_rift {
        let mid = cam.at(V::ZERO);
        d.circle(
            mid.0,
            mid.1,
            cam.len(c.rift.arena_radius),
            Color::Shadow,
            Color::Rift,
            4,
            1000,
        );
    } else {
        let tile = cam.len(256);
        let (ox, oy) = cam.at(V::ZERO);
        d.ground(ox.rem_euclid(tile), oy.rem_euclid(tile), tile);
        // a wash of the ground colour keeps the texture calm under the action
        d.rect(0, 0, cam.w, cam.h, Color::Ground, 450);
        // the edge of the map
        let half = c.run.map_half * SUB;
        let corners = [
            V::new(-half, -half),
            V::new(half, -half),
            V::new(half, half),
            V::new(-half, half),
            V::new(-half, -half),
        ];
        let pts: Vec<(i32, i32)> = corners.iter().map(|p| cam.at(*p)).collect();
        d.line(&pts, Color::Rock, 6, 700);
    }

    // rift portal
    if let Some(r) = &w.rift {
        let at = cam.at(r.pos);
        let size = cam.len(c.events.rift_radius * 3);
        d.sprite(sprite("fx/rift"), at.0, at.1, size, false, 1000);
        // a marker at the screen edge points to a rift that is off screen
        if !cam.on(at, 0) {
            edge_marker(&mut d, &cam, at, Color::Rift);
        }
    }

    // pickups
    for p in &w.pickups {
        let at = cam.at(p.pos);
        if !cam.on(at, 40) {
            continue;
        }
        let (name, size) = match p.kind {
            PickupKind::Xp => ("fx/xp_shard", if p.value > 4 { 28 } else { 22 }),
            PickupKind::Gold => ("fx/gold_chip", 20),
            PickupKind::Health => ("fx/health_cell", 24),
            PickupKind::Cache => ("fx/cache", 36),
            PickupKind::Ticket => ("fx/ticket", 26),
        };
        d.sprite(sprite(name), at.0, at.1, cam.len(size), false, 1000);
    }

    // obstacles
    if !w.in_rift {
        for o in &c.run.obstacles {
            let at = cam.at(V::new(o.x * SUB, o.y * SUB));
            if !cam.on(at, cam.len(o.r * 3)) {
                continue;
            }
            let name = if o.kind == "wreck" {
                "ground/wreck"
            } else {
                "ground/rock_pillar"
            };
            d.sprite(sprite(name), at.0, at.1, cam.len(o.r * 5 / 2), false, 1000);
        }
    }

    // the players' own effects, faint so they never hide a threat
    for e in &w.effects {
        let at = cam.at(e.pos);
        let fade = (e.life * 1000 / e.full.max(1)).clamp(0, 1000);
        match e.kind {
            EffectKind::Ring => {
                let grow = if calm { 1000 } else { 1000 - fade / 2 };
                d.circle(
                    at.0,
                    at.1,
                    cam.len(e.radius) * grow / 1000,
                    Color::None,
                    Color::Ring,
                    3,
                    fade * 6 / 10,
                );
            }
            EffectKind::Wedge => {
                let dir = turn_of(e.dir);
                let half = e.width.min(fx::TURN) / 2;
                d.wedge(
                    at.0,
                    at.1,
                    cam.len(e.radius),
                    dir - half,
                    dir + half,
                    Color::Ring,
                    fade * 3 / 10,
                );
            }
            EffectKind::Beam => {
                let end = cam.at(e.pos.add(e.dir.with_len(e.shot_range * SUB)));
                d.line(&[at, end], Color::Ring, cam.len(e.radius * 2), 450);
            }
            EffectKind::Chain => {
                let pts: Vec<(i32, i32)> = e.points.iter().map(|p| cam.at(*p)).collect();
                d.line(&pts, Color::Ring, 3, fade * 8 / 10);
            }
            EffectKind::Bomb => {
                d.circle(at.0, at.1, cam.len(8), Color::Bolt, Color::Ink, 2, 900);
                d.circle(at.0, at.1, cam.len(e.radius), Color::None, Color::Ring, 1, 300);
            }
            EffectKind::Orbit => {
                let n = e.count.max(1);
                for b in 0..n {
                    let p = e.pos.add(crate::geom::from_angle(
                        e.angle + fx::TURN * b / n,
                        e.shot_range * SUB,
                    ));
                    let q = cam.at(p);
                    d.circle(q.0, q.1, cam.len(e.radius), Color::Bolt, Color::Ink, 2, 900);
                }
            }
            EffectKind::Turret => {
                d.circle(at.0, at.1, cam.len(12), Color::Panel, Color::Ink, 3, 1000);
                d.circle(at.0, at.1, cam.len(5), Color::Bolt, Color::Ink, 1, 1000);
            }
        }
    }

    // player shots
    for s in &w.shots {
        let at = cam.at(s.pos);
        if cam.on(at, 20) {
            d.circle(at.0, at.1, cam.len(s.radius), Color::Bolt, Color::Ink, 2, 850);
        }
    }

    // enemies, back to front
    let mut order: Vec<usize> = (0..w.foes.len()).collect();
    order.sort_by_key(|&i| (w.foes[i].pos.y, w.foes[i].id));
    for i in order {
        let f = &w.foes[i];
        let e = &c.enemies[f.kind];
        let at = cam.at(f.pos);
        let size = cam.len(e.size);
        if !cam.on(at, size) {
            if e.boss || e.elite {
                edge_marker(&mut d, &cam, at, Color::Boss);
            }
            continue;
        }
        let folder = if e.boss { "bosses" } else { "enemies" };
        d.circle(
            at.0,
            at.1 + size * 3 / 10,
            size * 3 / 10,
            Color::Shadow,
            Color::None,
            0,
            500,
        );
        d.sprite(
            sprite(&format!("{folder}/{}", e.id)),
            at.0,
            at.1,
            size,
            f.facing_left,
            1000,
        );
        if f.flash > 0 && !calm {
            d.circle(at.0, at.1, cam.len(e.radius), Color::Flash, Color::None, 0, 450);
        }
        if e.elite && !e.boss {
            hp_bar(
                &mut d,
                at.0 - size / 3,
                at.1 - size / 2 - 8,
                size * 2 / 3,
                f.hp,
                f.max_hp,
                Color::Boss,
            );
        }
    }

    // players
    for p in &w.players {
        // the partner glides after a correction (T13)
        let pos = match g.partner_drawn {
            Some(drawn) if p.seat != g.seat && w.players.len() > 1 => drawn,
            _ => p.pos,
        };
        let at = cam.at(pos);
        let size = cam.len(64);
        let ch = &c.characters[p.character];
        let alpha = if p.alive { 1000 } else { 400 };
        d.circle(
            at.0,
            at.1 + size * 2 / 5,
            size * 2 / 5,
            Color::None,
            seat_color(p.seat),
            3,
            alpha,
        );
        d.sprite(
            sprite(&player_picture(w, p, &ch.id)),
            at.0,
            at.1,
            size,
            p.facing.x < 0,
            alpha,
        );
        if p.hurt_flash > 0 && !calm {
            d.circle(at.0, at.1, size / 3, Color::Hp, Color::None, 0, 350);
        }
        if p.alive {
            let max = w.max_hp(c, p.seat);
            hp_bar(
                &mut d,
                at.0 - size / 2,
                at.1 + size / 2 + 6,
                size,
                p.hp,
                max,
                Color::Hp,
            );
        } else {
            let need = crate::geom::ticks(c.run.revive_ms).max(1);
            let r = cam.len(c.run.revive_radius);
            d.circle(at.0, at.1, r, Color::None, Color::Heal, 2, 600);
            if p.revive_progress > 0 {
                d.wedge(
                    at.0,
                    at.1,
                    r,
                    -fx::QUARTER_TURN,
                    -fx::QUARTER_TURN + fx::TURN * p.revive_progress / need,
                    Color::Heal,
                    350,
                );
            }
        }
        if w.players.len() > 1 {
            d.text(
                at.0,
                at.1 - size / 2 - 10,
                14,
                seat_color(p.seat),
                Align::Center,
                &g.text.fill("hud.seat", &[("n", (p.seat + 1).to_string())]),
            );
            if p.seat != g.seat && !cam.on(at, 0) {
                edge_marker(&mut d, &cam, at, seat_color(p.seat));
            }
        }
    }

    // enemy shots last: one shape, ringed in ink, never under anything
    for b in &w.bolts {
        let at = cam.at(b.pos);
        if cam.on(at, 20) {
            d.sprite(
                sprite("fx/enemy_shot"),
                at.0,
                at.1,
                cam.len(b.radius * 3),
                false,
                1000,
            );
        }
    }

    hud(&mut d, g, w, c);
    d.finish()
}

/// Ticks each walk frame shows (T6): about eight frames a second.
pub const WALK_FRAME_MS: i32 = 125;

/// The picture for a player: a walk frame while moving, the standing
/// picture otherwise.
pub fn player_picture(w: &World, p: &crate::world::Player, id: &str) -> String {
    if p.moving && p.alive && p.dash_left == 0 {
        let step = crate::geom::ticks(WALK_FRAME_MS).max(1) as u32;
        format!("players/{id}_walk{}", (w.tick / step) % 4)
    } else {
        format!("players/{id}")
    }
}

fn turn_of(v: V) -> i32 {
    // the angle of v in turns, by bisection on the rotation of (1, 0)
    if v.is_zero() {
        return 0;
    }
    let mut best = 0;
    let mut best_dot = i64::MIN;
    let steps = 64;
    for k in 0..steps {
        let a = fx::TURN * k / steps;
        let u = crate::geom::from_angle(a, 1024);
        let dot = u.dot(v);
        if dot > best_dot {
            best_dot = dot;
            best = a;
        }
    }
    best
}

fn edge_marker(d: &mut DrawList, cam: &Cam, at: (i32, i32), color: Color) {
    let m = 22;
    let x = at.0.clamp(m, cam.w - m);
    let y = at.1.clamp(m + 30, cam.h - m);
    d.circle(x, y, 9, color, Color::Ink, 3, 900);
}

fn hp_bar(d: &mut DrawList, x: i32, y: i32, w: i32, hp: i32, max: i32, color: Color) {
    let h = 6;
    d.rect(x - 1, y - 1, w + 2, h + 2, Color::Ink, 900);
    d.rect(x, y, w, h, Color::HpBack, 1000);
    let fill = (w as i64 * hp.max(0) as i64 / max.max(1) as i64) as i32;
    d.rect(x, y, fill, h, color, 1000);
}

/// How long a dash prompt shows at the start of a run, until the first dash.
pub const DASH_PROMPT_MS: i32 = 12_000;

fn hud(d: &mut DrawList, g: &Game, w: &World, c: &Content) {
    let vw = g.view_w;
    // experience across the top
    let need = World::xp_needed(c, w.level).max(1);
    d.rect(0, 0, vw, 10, Color::HpBack, 1000);
    d.rect(
        0,
        0,
        (vw as i64 * w.xp as i64 / need as i64) as i32,
        10,
        Color::Xp,
        1000,
    );
    let left = (c.run.length_s - w.seconds()).max(0);
    let remaining = if w.final_spawned {
        g.text.get("hud.final")
    } else {
        clock(left)
    };
    d.text(vw / 2, 34, 22, Color::Text, Align::Center, &remaining);
    d.text(
        16,
        34,
        18,
        Color::Text,
        Align::Left,
        &g.text.fill("hud.level", &[("n", w.level.to_string())]),
    );
    let me = &w.players[g.seat.min(w.players.len() - 1)];
    d.text(
        vw - 16,
        34,
        18,
        Color::Text,
        Align::Right,
        &g.text.fill("hud.kills", &[("n", me.record.kills.to_string())]),
    );
    // the connection, so players can say what they saw (T13)
    if let Some(n) = g.session.as_ref().filter(|n| n.running) {
        let st = n.stats;
        let wait_tenths = st.stalls as i64 * 10 / crate::geom::TPS as i64;
        d.text(
            vw - 16,
            58,
            14,
            Color::Rock,
            Align::Right,
            &g.text.fill(
                "hud.net",
                &[
                    ("ms", (st.rtt_us / 1000).to_string()),
                    ("fixes", st.rollbacks.to_string()),
                    ("wait", format!("{}.{}", wait_tenths / 10, wait_tenths % 10)),
                ],
            ),
        );
    }
    if w.in_rift {
        let s = (w.rift_left / crate::geom::TPS).max(0);
        d.text(
            vw / 2,
            62,
            16,
            Color::Rift,
            Align::Center,
            &g.text.fill("hud.rift", &[("t", s.to_string())]),
        );
    }
    // the boss's health
    if let Some(f) = w.foes.iter().find(|f| c.enemies[f.kind].boss) {
        let bw = (vw * 6 / 10).min(640);
        let x = (vw - bw) / 2;
        let y = if w.in_rift { 80 } else { 58 };
        hp_bar(d, x, y, bw, f.hp, f.max_hp, Color::Boss);
        d.text(
            vw / 2,
            y + 22,
            14,
            Color::Text,
            Align::Center,
            &g.text.name(&c.enemies[f.kind].id),
        );
    }
    // a character that can dash is told so, and sees when it is ready (T11)
    let ch = &c.characters[me.character];
    if ch.trait_ == "dash" && me.alive {
        let full = crate::geom::ticks(ch.dash_cooldown_ms).max(1);
        let ready = me.dash_cd == 0;
        let y = 60;
        let bw = 120;
        d.rect(16, y - 4, bw, 8, Color::HpBack, 1000);
        d.rect(16, y - 4, bw * (full - me.dash_cd) / full, 8, Color::Xp, 1000);
        d.text(
            16 + bw + 10,
            y,
            16,
            if ready { Color::Text } else { Color::Rock },
            Align::Left,
            &g.text.get(if ready { "hud.dash_ready" } else { "hud.dash_wait" }),
        );
        if me.record.dashes == 0 && w.tick < crate::geom::ticks(DASH_PROMPT_MS) as u32 {
            d.text(
                vw / 2,
                g.view_h / 2 + 70,
                20,
                Color::Text,
                Align::Center,
                &g.text.get("hud.dash_prompt"),
            );
        }
    }
    if let Some(b) = &w.banner {
        let y = g.view_h / 4;
        d.rect(0, y - 26, vw, 52, Color::Panel, 700);
        d.text(vw / 2, y, 24, Color::Text, Align::Center, &g.text.get(&b.key));
    }
    if g.paused {
        d.rect(0, 0, vw, g.view_h, Color::Panel, 500);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::walk;
    use crate::game::tests::game;

    #[test]
    fn a_run_frame_is_a_well_formed_list() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        for f in 0..600 {
            g.advance(f * 16_667, crate::game::Raw::default());
        }
        let list = frame(&g);
        assert!(walk(&list).unwrap() > 5);
    }

    #[test]
    fn enemy_shots_draw_after_everything_but_the_hud() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        let c = g.content.clone();
        let w = g.world.as_mut().unwrap();
        w.add_test_bolt(V::new(0, 40 * SUB));
        let kind = c.enemy_index("skitter");
        w.spawn_foe(&c, kind, V::new(30 * SUB, 0));
        let list = frame(&g);
        let shot = sprite("fx/enemy_shot");
        let foe = sprite("enemies/skitter");
        // find the op positions of the two sprites
        let pos = |want: i32| {
            let mut at = 0;
            let mut found = None;
            while at < list.len() {
                let code = list[at];
                if code == crate::draw::Op::Sprite as i32 && list[at + 1] == want {
                    found = Some(at);
                }
                let n = match code {
                    0 => 1,
                    1 => 6,
                    2 => 7,
                    3 => 6,
                    4 => 4 + list[at + 4] as usize * 2,
                    5 => 6 + list[at + 6] as usize,
                    6 => 3,
                    _ => 7,
                };
                at += 1 + n;
            }
            found
        };
        assert!(pos(shot).unwrap() > pos(foe).unwrap());
    }

    #[test]
    fn a_moving_player_cycles_four_walk_frames_and_a_still_one_stands() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        let c = g.content.clone();
        let w = g.world.as_mut().unwrap();
        let still = player_picture(w, &w.players[0], "pathfinder");
        assert_eq!(still, "players/pathfinder");
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..60 {
            w.step(
                &c,
                &[crate::world::Input {
                    mx: 127,
                    ..Default::default()
                }],
            );
            seen.insert(player_picture(w, &w.players[0], "pathfinder"));
        }
        let want: std::collections::BTreeSet<String> =
            (0..4).map(|k| format!("players/pathfinder_walk{k}")).collect();
        assert_eq!(seen, want);
        for name in &want {
            assert!(sprite(name) >= 0, "{name} is a known picture");
        }
    }

    fn texts(list: &[i32]) -> Vec<String> {
        // the text op: code 5, x, y, size, colour, align, count, then codes
        let mut out = Vec::new();
        let mut at = 0;
        while at < list.len() {
            let code = list[at];
            let n = match code {
                0 => 1,
                1 => 6,
                2 => 7,
                3 => 6,
                4 => 4 + list[at + 4] as usize * 2,
                5 => 6 + list[at + 6] as usize,
                6 => 3,
                _ => 7,
            };
            if code == 5 {
                let count = list[at + 6] as usize;
                let s: String = list[at + 7..at + 7 + count]
                    .iter()
                    .filter_map(|&u| char::from_u32(u as u32))
                    .collect();
                out.push(s);
            }
            at += 1 + n;
        }
        out
    }

    #[test]
    fn vector_is_reminded_to_dash_and_pathfinder_is_not() {
        let mut g = game();
        g.save.unlocked.push("vector".into());
        g.act("hangar.pick", "vector").unwrap();
        g.act("run.start", "").unwrap();
        g.advance(0, crate::game::Raw::default());
        let shown = texts(&frame(&g));
        assert!(shown.contains(&g.text.get("hud.dash_prompt")), "{shown:?}");
        assert!(shown.contains(&g.text.get("hud.dash_ready")));
        g.act("run.quit", "").unwrap();
        g.act("hangar.pick", "pathfinder").unwrap();
        g.act("run.start", "").unwrap();
        let shown = texts(&frame(&g));
        assert!(!shown.contains(&g.text.get("hud.dash_prompt")));
        assert!(!shown.contains(&g.text.get("hud.dash_ready")));
    }

    #[test]
    fn the_zoom_keeps_a_phone_wide_enough() {
        assert_eq!(zoom(1280, 720), 1000);
        assert_eq!(zoom(400, 800), 450);
    }
}
