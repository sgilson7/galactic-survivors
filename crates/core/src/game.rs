//! The whole game as the page sees it: one object holding the save, the
//! screen, the run and the co-op session. The page calls `advance` every
//! display frame with the time and the raw input, draws `draw_list`, and
//! renders `ui`. Every button comes back through `act(name, value)`; an
//! unknown action is an error.

use crate::content::Content;
use crate::meta::{Payout, Pull, Save};
use crate::online::{self, Event, Session};
use crate::rng::Rng;
use crate::text::Text;
use crate::ticks::Clock;
use crate::world::{Input, Loadout, Phase, World, AIM, CARD_REROLL, DASH};

/// Input bits the page sends, by action name (`data/controls.json` names an
/// action for each key).
pub const BITS: &[(&str, u32)] = &[
    ("up", 1),
    ("down", 2),
    ("left", 4),
    ("right", 8),
    ("dash", 16),
    ("card1", 32),
    ("card2", 64),
    ("card3", 128),
    ("reroll", 256),
    ("pause", 512),
    ("inspect", 1024),
];

/// Screen pixels a touch must drag before it steers.
pub const STICK_DEAD_ZONE: i64 = 12;

fn bit(name: &str) -> u32 {
    BITS.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Hangar,
    Shop,
    Cache,
    Achievements,
    Settings,
    Coop,
    Run,
    Results,
    Help,
    Gear,
    Codex,
}

impl Screen {
    pub fn name(self) -> &'static str {
        match self {
            Screen::Title => "title",
            Screen::Hangar => "hangar",
            Screen::Shop => "shop",
            Screen::Cache => "cache",
            Screen::Achievements => "achievements",
            Screen::Settings => "settings",
            Screen::Coop => "coop",
            Screen::Run => "run",
            Screen::Results => "results",
            Screen::Help => "help",
            Screen::Gear => "gear",
            Screen::Codex => "codex",
        }
    }

    fn from(name: &str) -> Option<Screen> {
        Some(match name {
            "title" => Screen::Title,
            "hangar" => Screen::Hangar,
            "shop" => Screen::Shop,
            "cache" => Screen::Cache,
            "achievements" => Screen::Achievements,
            "settings" => Screen::Settings,
            "coop" => Screen::Coop,
            "help" => Screen::Help,
            "gear" => Screen::Gear,
            "codex" => Screen::Codex,
            _ => return None,
        })
    }
}

/// The raw input of one display frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Raw {
    pub bits: u32,
    pub stick_x: i32,
    pub stick_y: i32,
    pub pointer_x: i32,
    pub pointer_y: i32,
    pub pointer_on: bool,
}

pub struct Game {
    pub content: Content,
    pub text: Text,
    pub save: Save,
    pub rng: Rng,
    pub screen: Screen,
    pub world: Option<World>,
    pub seat: usize,
    pub session: Option<Session>,
    pub room: String,
    pub hosting: bool,
    pub peer_load: Option<Loadout>,
    pub net_error: String,
    pub build: String,
    pub view_w: i32,
    pub view_h: i32,
    clock: Clock,
    held: u32,
    last_bits: u32,
    pending_card: u8,
    pub paused: bool,
    /// The build overlay in a co-op run, which does not pause (T1).
    pub inspect: bool,
    /// The codex is showing, on its screen or over a run (T10).
    pub codex_open: bool,
    pub codex_tab: String,
    /// The gear piece chosen on the gear screen, 0 for none.
    pub gear_pick: u32,
    /// The achievement chosen on the grid.
    pub ach_pick: String,
    coop_due: i32,
    pub payout: Option<Payout>,
    pub pulls: Vec<Pull>,
    pub reveal_all: bool,
    pub notice: String,
    pub saved_since_run: bool,
    pub frame: u32,
}

impl Game {
    pub fn new(content_json: &str, strings_json: &str, seed: u64, build: &str) -> Result<Game, String> {
        let content = Content::parse(content_json)?;
        let text = Text::parse(strings_json)?;
        let save = Save::new(&content, seed);
        Ok(Game {
            content,
            text,
            save,
            rng: Rng::new(seed),
            screen: Screen::Title,
            world: None,
            seat: 0,
            session: None,
            room: String::new(),
            hosting: false,
            peer_load: None,
            net_error: String::new(),
            build: build.to_string(),
            view_w: 1280,
            view_h: 720,
            clock: Clock::default(),
            held: 0,
            last_bits: 0,
            pending_card: 0,
            paused: false,
            inspect: false,
            codex_open: false,
            codex_tab: "abilities".into(),
            gear_pick: 0,
            ach_pick: String::new(),
            coop_due: 0,
            payout: None,
            pulls: Vec::new(),
            reveal_all: false,
            notice: String::new(),
            saved_since_run: true,
            frame: 0,
        })
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.view_w = w.max(1);
        self.view_h = h.max(1);
    }

    pub fn coop_running(&self) -> bool {
        self.session.as_ref().map(|n| n.running).unwrap_or(false)
    }

    // ---- input ---------------------------------------------------------------

    /// The local seat's input for a tick, from keys or the stick and the
    /// pointer. The pointer aims from the middle of the screen, where the
    /// camera keeps the local player.
    pub fn local_input(&self, raw: &Raw, bits: u32, card: u8) -> Input {
        let mut dx = 0;
        let mut dy = 0;
        if bits & bit("left") != 0 {
            dx -= 1;
        }
        if bits & bit("right") != 0 {
            dx += 1;
        }
        if bits & bit("up") != 0 {
            dy -= 1;
        }
        if bits & bit("down") != 0 {
            dy += 1;
        }
        let mut mv = crate::geom::V::new(dx * 1000, dy * 1000);
        let stick = crate::geom::V::new(raw.stick_x, raw.stick_y);
        // a drag shorter than the dead zone (in screen pixels) is a tap
        if mv.is_zero() && stick.len() >= STICK_DEAD_ZONE {
            mv = stick;
        }
        let mv = mv.with_len(127);
        let mut flags = 0;
        let mut aim = crate::geom::V::ZERO;
        if raw.pointer_on {
            aim = crate::geom::V::new(raw.pointer_x - self.view_w / 2, raw.pointer_y - self.view_h / 2)
                .with_len(127);
            if !aim.is_zero() {
                flags |= AIM;
            }
        }
        if bits & bit("dash") != 0 {
            flags |= DASH;
        }
        Input {
            mx: mv.x as i8,
            my: mv.y as i8,
            ax: aim.x as i8,
            ay: aim.y as i8,
            flags,
            card,
        }
    }

    fn card_from_bits(bits: u32) -> u8 {
        if bits & bit("card1") != 0 {
            1
        } else if bits & bit("card2") != 0 {
            2
        } else if bits & bit("card3") != 0 {
            3
        } else if bits & bit("reroll") != 0 {
            CARD_REROLL
        } else {
            0
        }
    }

    /// One display frame: run the ticks the time allows.
    pub fn advance(&mut self, now_us: i64, raw: Raw) {
        self.frame = self.frame.wrapping_add(1);
        let pressed = raw.bits & !self.last_bits;
        self.last_bits = raw.bits;
        self.held = raw.bits;
        if self.screen != Screen::Run || self.world.is_none() {
            self.clock = Clock::default();
            return;
        }
        if pressed & (bit("pause") | bit("inspect")) != 0 {
            self.toggle_overlay();
        }
        let tick_bits = self.clock.frame(now_us, raw.bits);
        let local_offer = self
            .world
            .as_ref()
            .map(|w| w.players[self.seat].offer.is_some())
            .unwrap_or(false);
        if local_offer && self.pending_card == 0 {
            self.pending_card = Game::card_from_bits(pressed);
        }
        if self.coop_running() {
            self.advance_coop(&raw, tick_bits.len());
        } else if !self.paused {
            for b in tick_bits {
                let card = std::mem::take(&mut self.pending_card);
                let input = self.local_input(&raw, b, card);
                let content = &self.content;
                if let Some(w) = self.world.as_mut() {
                    w.step(content, &[input]);
                }
            }
        }
        self.after_tick();
    }

    fn advance_coop(&mut self, raw: &Raw, budget: usize) {
        let card = std::mem::take(&mut self.pending_card);
        let input = self.local_input(raw, self.held, card);
        let Some(link) = self.session.as_mut() else { return };
        link.local(input.pack());
        // the clock says how many ticks are due; ticks that could not run
        // for want of the partner's input stay due, a few at most, so a
        // late copy catches up and no copy runs ahead of real time
        self.coop_due = (self.coop_due + budget as i32).min(crate::ticks::MAX_TICKS_PER_FRAME as i32);
        while self.coop_due > 0 {
            let Some(link) = self.session.as_mut() else { return };
            let Some(keys) = link.ready() else { break };
            let inputs = [Input::unpack(keys[0]), Input::unpack(keys[1])];
            let content = &self.content;
            let Some(w) = self.world.as_mut() else { return };
            w.step(content, &inputs);
            let sum = w.sum();
            if let Some(link) = self.session.as_mut() {
                link.ran(sum);
            }
            self.coop_due -= 1;
        }
        if card != 0 {
            // a card action rides on the next input only once
            if let Some(link) = self.session.as_mut() {
                link.local(Input { card: 0, ..input }.pack());
            }
        }
    }

    fn after_tick(&mut self) {
        let lost_partner = self
            .session
            .as_ref()
            .map(|n| matches!(n.status, online::LEFT_ROOM | online::DESYNC) && self.world.is_some())
            .unwrap_or(false);
        let over = self
            .world
            .as_ref()
            .map(|w| w.phase != Phase::Playing)
            .unwrap_or(false);
        if over || lost_partner {
            if lost_partner {
                self.notice = self
                    .session
                    .as_ref()
                    .map(|n| n.status.to_string())
                    .unwrap_or_default();
            }
            self.end_run();
        }
    }

    /// Opens or closes the build overlay: a pause in a solo run, a view
    /// that leaves the run going in co-op.
    pub fn toggle_overlay(&mut self) {
        if self.coop_running() {
            self.inspect = !self.inspect;
        } else {
            self.paused = !self.paused;
        }
        self.codex_open = false;
    }

    pub fn overlay_open(&self) -> bool {
        self.paused || self.inspect
    }

    pub fn end_run(&mut self) {
        let Some(w) = self.world.take() else { return };
        let payout = self.save.finish_run(&self.content, &w, self.seat);
        self.payout = Some(payout);
        self.saved_since_run = false;
        self.screen = Screen::Results;
        self.paused = false;
        self.inspect = false;
        self.codex_open = false;
        if let Some(link) = self.session.as_mut() {
            link.running = false;
        }
    }

    // ---- starting runs -------------------------------------------------------

    pub fn start_solo(&mut self) -> Result<(), String> {
        let ch = self.save.character.clone();
        if !self.save.unlocked.contains(&ch) {
            return Err(self.text.get("hangar.locked"));
        }
        let load = self.save.loadout(&self.content, &ch);
        let seed = self.rng.next_u64();
        self.world = Some(World::new(&self.content, seed, &[load]));
        self.seat = 0;
        self.begin_run();
        Ok(())
    }

    fn begin_run(&mut self) {
        self.screen = Screen::Run;
        self.clock = Clock::default();
        self.paused = false;
        self.inspect = false;
        self.codex_open = false;
        self.pending_card = 0;
        self.payout = None;
    }

    pub fn my_load(&self) -> Loadout {
        self.save.loadout(&self.content, &self.save.character)
    }

    // ---- co-op ---------------------------------------------------------------

    /// Hosting: a new session as seat 0 and a room code from the page's
    /// number. Returns the room name for the transport.
    pub fn host(&mut self, n: u64, app_id: &str) -> String {
        let code = online::make_code(n);
        self.session = Some(Session::new(0, &self.build));
        self.hosting = true;
        self.peer_load = None;
        self.net_error.clear();
        self.room = code.clone();
        online::room_name(app_id, &code)
    }

    /// Joining by a typed code (or none, for a pasted invitation). Returns
    /// the room name, or None if the code cannot be one.
    pub fn join(&mut self, code: &str, app_id: &str) -> Option<String> {
        let cleaned = if code.is_empty() {
            String::new()
        } else {
            online::clean_code(code)?
        };
        self.session = Some(Session::new(1, &self.build));
        self.hosting = false;
        self.peer_load = None;
        self.net_error.clear();
        self.room = cleaned.clone();
        Some(online::room_name(app_id, &cleaned))
    }

    pub fn net_peer(&mut self, peer: u32) {
        if let Some(n) = self.session.as_mut() {
            n.connected(peer);
        }
    }

    pub fn net_left(&mut self, peer: u32) {
        if let Some(n) = self.session.as_mut() {
            n.left(peer);
        }
        self.peer_load = None;
        self.after_tick();
    }

    pub fn net_message(&mut self, peer: u32, text: &str) {
        let was_lobby = self
            .session
            .as_ref()
            .map(|n| n.status == online::LOBBY)
            .unwrap_or(false);
        let Some(n) = self.session.as_mut() else { return };
        n.message(peer, text);
        let now_lobby = n.status == online::LOBBY;
        if now_lobby && !was_lobby && n.seat == 1 {
            let load = self.my_load();
            if let Some(n) = self.session.as_mut() {
                n.send_load(&load);
            }
        }
        let events = self.session.as_mut().map(|n| n.take_events()).unwrap_or_default();
        for e in events {
            match e {
                Event::PeerLoad(load) => self.peer_load = Some(load),
                Event::Start { seed, loads } => {
                    self.peer_load = loads.first().cloned();
                    self.world = Some(World::new(&self.content, seed, &loads));
                    self.seat = 1;
                    self.begin_run();
                }
            }
        }
    }

    pub fn net_outbox(&mut self) -> Vec<(u32, String)> {
        self.session.as_mut().map(|n| n.take_outbox()).unwrap_or_default()
    }

    pub fn set_net_error(&mut self, key: &str) {
        self.net_error = key.to_string();
    }

    pub fn leave_coop(&mut self) {
        if let Some(n) = self.session.as_mut() {
            n.leave();
        }
        self.hosting = false;
        self.peer_load = None;
        self.room.clear();
    }

    fn start_coop(&mut self) -> Result<(), String> {
        let peer = self
            .peer_load
            .clone()
            .ok_or_else(|| self.text.get("coop.no_partner"))?;
        let mine = self.my_load();
        let seed = self.rng.next_u64();
        let loads = vec![mine, peer];
        if let Some(n) = self.session.as_mut() {
            n.start(seed, loads.clone());
        }
        self.world = Some(World::new(&self.content, seed, &loads));
        self.seat = 0;
        self.begin_run();
        Ok(())
    }

    // ---- actions -------------------------------------------------------------

    fn refusal(&self, key: &str) -> String {
        self.text.get(key)
    }

    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.notice.clear();
        match name {
            "screen.show" => {
                let s = Screen::from(value).ok_or_else(|| format!("There is no screen \"{value}\"."))?;
                if self.screen == Screen::Run {
                    return Err(self.refusal("run.busy"));
                }
                self.screen = s;
                self.codex_open = s == Screen::Codex;
                if s == Screen::Cache {
                    self.pulls.clear();
                }
            }
            "hangar.pick" => {
                if self.content.character(value).is_none() {
                    return Err(format!("There is no character \"{value}\"."));
                }
                if !self.save.unlocked.iter().any(|c| c == value) {
                    return Err(self.refusal("hangar.locked"));
                }
                self.save.character = value.to_string();
                if self
                    .session
                    .as_ref()
                    .map(|n| n.seat == 1 && n.status == online::LOBBY)
                    .unwrap_or(false)
                {
                    let load = self.my_load();
                    if let Some(n) = self.session.as_mut() {
                        n.send_load(&load);
                    }
                }
            }
            "hangar.buy" => self
                .save
                .buy_character(&self.content, value)
                .map_err(|k| self.refusal(k))?,
            "gear.pick" => {
                let uid = self.uid(value)?;
                self.gear_pick = if self.gear_pick == uid { 0 } else { uid };
            }
            "gear.equip" => {
                let uid = self.uid(value)?;
                let ch = self.save.character.clone();
                self.save
                    .equip_auto(&self.content, &ch, uid)
                    .map_err(|k| self.refusal(k))?;
            }
            "gear.unequip" => {
                let uid = self.uid(value)?;
                let ch = self.save.character.clone();
                self.save.unequip(&ch, uid);
            }
            "gear.merge" => {
                let uid = self.uid(value)?;
                self.save.merge(&self.content, uid).map_err(|k| self.refusal(k))?;
            }
            "gear.merge_all" => {
                self.save.merge_all(&self.content);
                if self.save.piece(self.gear_pick).is_none() {
                    self.gear_pick = 0;
                }
            }
            "gear.scrap" => {
                let uid = self.uid(value)?;
                self.save.scrap(&self.content, uid).map_err(|k| self.refusal(k))?;
                self.gear_pick = 0;
            }
            "ach.pick" => {
                if self.content.achievement(value).is_none() {
                    return Err(format!("There is no achievement \"{value}\"."));
                }
                self.ach_pick = value.to_string();
            }
            "codex.open" => {
                self.codex_open = true;
            }
            "codex.close" => {
                self.codex_open = false;
            }
            "codex.tab" => {
                if !crate::view::CODEX_TABS.contains(&value) {
                    return Err(format!("There is no codex tab \"{value}\"."));
                }
                self.codex_tab = value.to_string();
            }
            "run.inspect" => {
                if self.world.is_some() {
                    self.toggle_overlay();
                }
            }
            "run.start" => self.start_solo()?,
            "run.pause" => {
                if !self.coop_running() {
                    self.paused = true;
                }
            }
            "run.resume" => {
                self.paused = false;
                self.inspect = false;
                self.codex_open = false;
            }
            "run.quit" => {
                if self.world.is_some() {
                    if self.coop_running() {
                        self.leave_coop();
                    }
                    self.end_run();
                }
            }
            "card.take" | "card.banish" => {
                let k: u8 = value.parse().map_err(|_| format!("No card \"{value}\"."))?;
                if !(1..=3).contains(&k) {
                    return Err(format!("No card \"{value}\"."));
                }
                self.pending_card = if name == "card.take" { k } else { k + 3 };
            }
            "card.reroll" => self.pending_card = CARD_REROLL,
            "shop.buy" => self
                .save
                .buy_rank(&self.content, value)
                .map_err(|k| self.refusal(k))?,
            "cache.pull" => {
                let n: i32 = if value == "10" { 10 } else { 1 };
                self.pulls = self.save.pull(&self.content, n).map_err(|k| self.refusal(k))?;
                self.reveal_all = false;
            }
            "cache.reveal" => self.reveal_all = true,
            "settings.autoaim" => self.save.settings.autoaim = !self.save.settings.autoaim,
            "settings.motion" => self.save.settings.reduced_motion = !self.save.settings.reduced_motion,
            "settings.numbers" => self.save.settings.damage_numbers = !self.save.settings.damage_numbers,
            "save.loaded" => self.load_save(value)?,
            "save.downloaded" => {
                self.saved_since_run = true;
                self.notice = "save.done".into();
            }
            "coop.start" => self.start_coop()?,
            "coop.leave" => self.leave_coop(),
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    fn uid(&self, value: &str) -> Result<u32, String> {
        value
            .parse::<u32>()
            .ok()
            .filter(|u| self.save.piece(*u).is_some())
            .ok_or_else(|| self.refusal("gear.unknown"))
    }

    pub fn save_text(&self) -> String {
        self.save.to_json()
    }

    pub fn load_save(&mut self, text: &str) -> Result<(), String> {
        let save = Save::from_json(&self.content, text).map_err(|k| self.text.get(k))?;
        self.save = save;
        self.saved_since_run = true;
        self.notice = "save.loaded".into();
        Ok(())
    }

    /// Action bits by name, for the page's key map.
    pub fn bits_json() -> String {
        let m: std::collections::BTreeMap<&str, u32> = BITS.iter().copied().collect();
        serde_json::to_string(&m).expect("bits serialise")
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn game() -> Game {
        Game::new(
            include_str!("../../../data/content.json"),
            include_str!("../../../data/strings.json"),
            11,
            "test",
        )
        .expect("game starts")
    }

    /// Passes everything each game left in its outbox to the other.
    fn wire(a: &mut Game, b: &mut Game) {
        for (_, text) in a.net_outbox() {
            b.net_message(1, &text);
        }
        for (_, text) in b.net_outbox() {
            a.net_message(2, &text);
        }
    }

    #[test]
    fn two_games_play_one_co_op_run_and_hold_the_same_world() {
        let mut host = game();
        let mut join = game();
        host.host(7, "gs");
        join.join("ABCDE", "gs").unwrap();
        host.net_peer(2);
        join.net_peer(1);
        wire(&mut host, &mut join);
        wire(&mut host, &mut join);
        assert!(host.peer_load.is_some(), "the host heard the joiner's loadout");
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        assert_eq!(join.screen, Screen::Run);
        assert_eq!((host.seat, join.seat), (0, 1));
        let mut same_tick_checks = 0;
        let mut most = 0;
        for f in 0..1800i64 {
            let raw = Raw {
                bits: if (f / 40) % 2 == 0 { 8 } else { 2 },
                ..Raw::default()
            };
            host.advance(f * 16_667, raw);
            join.advance(f * 16_667 + 3_000, Raw::default());
            wire(&mut host, &mut join);
            for g in [&mut host, &mut join] {
                if g.world
                    .as_ref()
                    .map(|w| w.players[g.seat].offer.is_some())
                    .unwrap_or(false)
                {
                    g.act("card.take", "1").unwrap();
                }
            }
            if f == 300 {
                // five seconds of frames: about five seconds of ticks, never
                // more (the run starts a few ticks in)
                let t = host.world.as_ref().map(|w| w.tick).unwrap_or(0);
                assert!(
                    (280..=310).contains(&t),
                    "after 300 frames the run is at tick {t}"
                );
            }
            if let (Some(a), Some(b)) = (host.world.as_ref(), join.world.as_ref()) {
                most = most.max(a.tick);
                if a.tick == b.tick {
                    assert_eq!(a.sum(), b.sum(), "frame {f}");
                    same_tick_checks += 1;
                }
            }
        }
        // a tick a frame, never more on average
        assert!(
            most > 600 && most <= 1800,
            "the run advanced at the clock's rate ({most} ticks)"
        );
        assert!(same_tick_checks > 100);
        for g in [&host, &join] {
            let status = g.session.as_ref().unwrap().status;
            assert!(status == online::PLAYING, "{status}");
        }
    }

    #[test]
    fn the_build_pauses_a_solo_run_but_not_a_co_op_one() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        let inspect = Raw {
            bits: bit("inspect"),
            ..Raw::default()
        };
        g.advance(0, inspect);
        assert!(g.paused && g.overlay_open());
        let t = g.world.as_ref().unwrap().tick;
        for f in 1..60 {
            g.advance(f * 16_667, Raw::default());
        }
        assert_eq!(
            g.world.as_ref().unwrap().tick,
            t,
            "a paused solo run stands still"
        );
        g.act("run.resume", "").unwrap();
        assert!(!g.overlay_open());

        let mut host = game();
        let mut join = game();
        host.host(7, "gs");
        join.join("ABCDE", "gs").unwrap();
        host.net_peer(2);
        join.net_peer(1);
        wire(&mut host, &mut join);
        wire(&mut host, &mut join);
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        host.act("run.inspect", "").unwrap();
        assert!(
            host.inspect && !host.paused,
            "co-op shows the build without pausing"
        );
        for f in 0..120i64 {
            host.advance(f * 16_667, Raw::default());
            join.advance(f * 16_667, Raw::default());
            wire(&mut host, &mut join);
        }
        assert!(host.world.as_ref().unwrap().tick > 60, "the co-op run kept going");
    }

    #[test]
    fn an_unknown_action_is_an_error() {
        let mut g = game();
        assert!(g.act("nope", "").is_err());
        assert!(g.act("screen.show", "nowhere").is_err());
    }

    #[test]
    fn a_locked_character_cannot_be_picked_until_bought() {
        let mut g = game();
        assert!(g.act("hangar.pick", "breaker").is_err());
        // breaker costs 500 gold in content.json
        g.save.gold = 500;
        g.act("hangar.buy", "breaker").unwrap();
        assert_eq!(g.save.gold, 0);
        g.act("hangar.pick", "breaker").unwrap();
        assert_eq!(g.save.character, "breaker");
    }

    #[test]
    fn a_quit_run_still_pays_gold_and_reaches_results() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        for f in 0..(60 * 30) {
            g.advance(f * 16_667, Raw::default());
            if let Some(w) = g.world.as_ref() {
                if w.players[0].offer.is_some() {
                    g.act("card.take", "1").unwrap();
                }
            }
        }
        g.act("run.quit", "").unwrap();
        assert_eq!(g.screen, Screen::Results);
        let p = g.payout.clone().unwrap();
        assert!(p.gold > 0);
        assert!(g.save.gold >= p.gold, "the payout reached the save");
        assert!(!g.saved_since_run);
    }
}
