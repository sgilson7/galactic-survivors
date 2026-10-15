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
    ("rigs", 2048),
];

/// Frames the partner glides for after a correction.
pub const EASE_FRAMES: u32 = 10;

/// Beyond this, the partner's drawn place jumps instead of gliding.
pub const PARTNER_SNAP_PX: i32 = 160;

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
    /// The partner's seat in the lobby, as they last sent it (T24).
    pub partner: Option<online::Seat>,
    /// The seat last sent to the partner, so a change is sent once (T23).
    sent: Option<online::Seat>,
    /// The joiner has pressed Ready; their own loadout change takes it back.
    pub ready: bool,
    /// The lobby's open tab: one of `online::PLACES`.
    pub lobby_tab: String,
    /// The invitation buttons are showing on the co-op screen.
    pub coop_more: bool,
    /// What Try again repeats: "host", a room code, or nothing.
    pub last_try: String,
    /// Leave was pressed: the session goes once its goodbye is sent.
    leaving: bool,
    pub net_error: String,
    pub build: String,
    pub view_w: i32,
    pub view_h: i32,
    clock: Clock,
    held: u32,
    last_bits: u32,
    pending_card: u8,
    /// A move made in the rig window, waiting for the next tick.
    pending_rig: u16,
    /// The page's clock at the last frame, for timing pings.
    now_us: i64,
    /// Where the partner is drawn in co-op: it eases toward where the world
    /// has them, so a correction is a glide, not a jump (T13).
    pub partner_drawn: Option<crate::geom::V>,
    /// Frames left to ease after the last correction; otherwise the
    /// partner is drawn where they are.
    ease_frames: u32,
    pub paused: bool,
    /// The build overlay in a co-op run, which does not pause (T1).
    pub inspect: bool,
    /// The codex is showing, on its screen or over a run (T10).
    pub codex_open: bool,
    pub codex_tab: String,
    /// The rig window is open over the run, which keeps going (T15).
    pub rigs_open: bool,
    /// The token picked up in the rig window, waiting for a place.
    pub rig_pick: Option<crate::rig::Spot>,
    /// The gear piece chosen on the gear screen, 0 for none.
    pub gear_pick: u32,
    /// The achievement chosen on the grid.
    pub ach_pick: String,
    pub payout: Option<Payout>,
    pub pulls: Vec<Pull>,
    pub reveal_all: bool,
    pub notice: String,
    pub saved_since_run: bool,
    /// The save as this browser last stored it (T25); None before the
    /// first store.
    stored: Option<Save>,
    /// Whether this browser keeps the save: unknown until the page tries.
    pub store_ok: Option<bool>,
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
            partner: None,
            sent: None,
            ready: false,
            lobby_tab: online::PLACES[0].into(),
            coop_more: false,
            last_try: String::new(),
            leaving: false,
            net_error: String::new(),
            build: build.to_string(),
            view_w: 1280,
            view_h: 720,
            clock: Clock::default(),
            held: 0,
            last_bits: 0,
            pending_card: 0,
            pending_rig: 0,
            now_us: 0,
            partner_drawn: None,
            ease_frames: 0,
            paused: false,
            inspect: false,
            codex_open: false,
            codex_tab: "abilities".into(),
            gear_pick: 0,
            rigs_open: false,
            rig_pick: None,
            ach_pick: String::new(),
            payout: None,
            pulls: Vec::new(),
            reveal_all: false,
            notice: String::new(),
            saved_since_run: true,
            stored: None,
            store_ok: None,
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
            rig: 0,
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
        self.now_us = now_us;
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
        if pressed & bit("rigs") != 0 {
            self.rigs_open = !self.rigs_open;
            self.rig_pick = None;
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
            self.advance_coop(&raw, &tick_bits);
        } else if !self.paused {
            for b in tick_bits {
                let card = std::mem::take(&mut self.pending_card);
                let rig = std::mem::take(&mut self.pending_rig);
                let input = Input {
                    rig,
                    ..self.local_input(&raw, b, card)
                };
                let content = &self.content;
                if let Some(w) = self.world.as_mut() {
                    w.step(content, &[input]);
                }
            }
        }
        self.after_tick();
    }

    /// One display frame of a co-op run (T13): this browser's input for the
    /// ticks ahead, the ticks the clock allows (after any rollback), and
    /// what to send.
    fn advance_coop(&mut self, raw: &Raw, tick_bits: &[u32]) {
        let bits = if tick_bits.is_empty() {
            raw.bits
        } else {
            tick_bits.iter().fold(0, |a, b| a | b)
        };
        let held = self.local_input(raw, bits, 0).pack() & online::HELD_MASK;
        let once = Input {
            card: std::mem::take(&mut self.pending_card),
            rig: std::mem::take(&mut self.pending_rig),
            ..Input::default()
        }
        .pack();
        let mut due = tick_bits.len() as u32;
        let (Some(link), Some(w)) = (self.session.as_mut(), self.world.as_mut()) else {
            return;
        };
        // keep pace: a browser far ahead of its partner gives up a tick now
        // and then, so neither is always running on guesses
        if link.ahead() > online::DELAY as i32 + 3 && self.frame.is_multiple_of(4) && due > 0 {
            due -= 1;
        }
        link.local(held, once);
        let corrections = link.stats.rollbacks;
        let content = &self.content;
        link.advance(
            w,
            due,
            &mut |w: &mut World, keys| w.step(content, &[Input::unpack(keys[0]), Input::unpack(keys[1])]),
            &|w: &World| w.sum(),
        );
        link.send_inputs();
        link.maybe_ping(self.now_us);
        if link.stats.rollbacks != corrections {
            self.ease_frames = EASE_FRAMES;
        }
        self.ease_partner();
    }

    /// For a few frames after a correction, moves the partner's drawn
    /// place a third of the way to the true one each frame; a long way (a
    /// rift, a revive) is a jump.
    fn ease_partner(&mut self) {
        self.ease_frames = self.ease_frames.saturating_sub(1);
        let Some(w) = self.world.as_ref() else { return };
        let Some(p) = w.players.get(1 - self.seat.min(1)) else {
            return;
        };
        let to = p.pos;
        let far = crate::geom::px(PARTNER_SNAP_PX) as i64;
        self.partner_drawn = Some(match self.partner_drawn {
            Some(at) if self.ease_frames > 0 && to.sub(at).len() <= far => at.add(to.sub(at).scale(1, 3)),
            _ => to,
        });
    }

    fn after_tick(&mut self) {
        let lost_partner = self
            .session
            .as_ref()
            .map(|n| matches!(n.status, online::LEFT_ROOM | online::DESYNC) && self.world.is_some())
            .unwrap_or(false);
        // in co-op the run is over once every input up to its end is
        // known: a guess may end a run that the true inputs do not
        let over = self
            .world
            .as_ref()
            .map(|w| {
                w.phase != Phase::Playing
                    && self
                        .session
                        .as_ref()
                        .filter(|n| n.running)
                        .map(|n| n.confirmed() >= w.tick)
                        .unwrap_or(true)
            })
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

    /// The rig window's click: pick up a token, put it down somewhere
    /// else (a swap, made on the next tick), or drop the pick.
    fn pick_spot(&mut self, spot: crate::rig::Spot) {
        use crate::rig::Spot;
        let Some(w) = self.world.as_ref() else { return };
        let p = &w.players[self.seat.min(w.players.len() - 1)];
        let held = match spot {
            Spot::Socket(r, s) => p.rigs.get(r).and_then(|r| r.sockets.get(s)).cloned(),
            Spot::Spare(k) => p.spare.get(k).cloned(),
        }
        .unwrap_or_default();
        self.rig_pick = match self.rig_pick {
            None if held.is_empty() => None,
            None => Some(spot),
            Some(from) if from == spot => None,
            Some(from) => {
                self.pending_rig = crate::rig::pack_move(from, spot);
                None
            }
        };
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
            // the last inputs go reliably, so the partner can confirm the end
            link.send_inputs_reliably();
            link.running = false;
            if link.status == online::PLAYING {
                // back in the lobby: seats are sent again (T24)
                link.status = online::LOBBY;
            }
        }
        self.sent = None;
        self.share();
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
        self.pending_rig = 0;
        self.rigs_open = false;
        self.rig_pick = None;
        self.partner_drawn = None;
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
        self.fresh_room();
        self.last_try = "host".into();
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
        self.fresh_room();
        self.last_try = cleaned.clone();
        self.room = cleaned.clone();
        Some(online::room_name(app_id, &cleaned))
    }

    fn fresh_room(&mut self) {
        self.partner = None;
        self.sent = None;
        self.ready = false;
        self.leaving = false;
        self.net_error.clear();
        self.lobby_tab = online::PLACES[0].into();
    }

    /// In a co-op room and not on the way out of it.
    pub fn in_room(&self) -> bool {
        self.session.is_some() && !self.leaving
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
        self.partner = None;
        self.after_tick();
    }

    pub fn net_message(&mut self, peer: u32, text: &str) {
        let was_lobby = self
            .session
            .as_ref()
            .map(|n| n.status == online::LOBBY)
            .unwrap_or(false);
        let Some(n) = self.session.as_mut() else { return };
        n.message(peer, text, self.now_us);
        if n.status == online::LOBBY && !was_lobby {
            self.sent = None;
            self.share();
        }
        let events = self.session.as_mut().map(|n| n.take_events()).unwrap_or_default();
        for e in events {
            match e {
                Event::PeerSeat(seat) => self.partner = Some(seat),
                Event::Start { seed, loads } => {
                    self.world = Some(World::new(&self.content, seed, &loads));
                    self.seat = 1;
                    self.begin_run();
                }
            }
        }
    }

    pub fn net_outbox(&mut self) -> Vec<(u32, String, bool)> {
        let out = self.session.as_mut().map(|n| n.take_outbox()).unwrap_or_default();
        if self.leaving {
            // the goodbye is on its way: the room is gone
            self.session = None;
            self.leaving = false;
        }
        out
    }

    pub fn set_net_error(&mut self, key: &str) {
        self.net_error = key.to_string();
    }

    pub fn leave_coop(&mut self) {
        if let Some(n) = self.session.as_mut() {
            n.leave();
            self.leaving = true;
        }
        self.hosting = false;
        self.partner = None;
        self.sent = None;
        self.ready = false;
        self.room.clear();
    }

    fn start_coop(&mut self) -> Result<(), String> {
        let partner = self
            .partner
            .clone()
            .ok_or_else(|| self.text.get("coop.no_partner"))?;
        if !partner.ready {
            return Err(self.text.get("coop.not_ready"));
        }
        let peer = partner.load;
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

    /// Where this player is, as the partner's lobby shows it.
    fn place(&self) -> String {
        match self.screen {
            Screen::Coop => self.lobby_tab.clone(),
            Screen::Hangar => "character".into(),
            Screen::Gear => "gear".into(),
            Screen::Shop => "shop".into(),
            _ => String::new(),
        }
    }

    /// Each player sends their seat whenever it differs from the one the
    /// partner last heard: a character, gear or Outfitter change on any
    /// screen reaches the run (T23), and so do the name, Ready and where
    /// they are (T24). The joiner's own loadout change takes back Ready.
    fn share(&mut self) {
        let Some((seat_no, lobby)) = self
            .session
            .as_ref()
            .map(|n| (n.seat, n.status == online::LOBBY && !n.running))
        else {
            return;
        };
        if !lobby || self.leaving {
            return;
        }
        let load = self.my_load();
        let changed = self.sent.as_ref().map(|s| s.load != load).unwrap_or(false);
        if seat_no == 1 && changed {
            self.ready = false;
        }
        let seat = online::Seat {
            load,
            name: self.save.name.clone(),
            ready: seat_no == 1 && self.ready,
            at: self.place(),
        };
        if self.sent.as_ref() == Some(&seat) {
            return;
        }
        if let Some(n) = self.session.as_mut() {
            n.send_seat(&seat);
        }
        self.sent = Some(seat);
    }

    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.notice.clear();
        let done = self.act_on(name, value);
        self.share();
        done
    }

    fn act_on(&mut self, name: &str, value: &str) -> Result<(), String> {
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
            "run.start" => {
                if self.in_room() {
                    return Err(self.refusal("coop.in_room"));
                }
                self.start_solo()?
            }
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
            "rig.open" => {
                self.rigs_open = self.world.is_some();
                self.rig_pick = None;
            }
            "rig.close" => {
                self.rigs_open = false;
                self.rig_pick = None;
            }
            "rig.pick" => {
                let spot =
                    crate::rig::Spot::parse(value).ok_or_else(|| format!("No rig place \"{value}\"."))?;
                self.pick_spot(spot);
            }
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
            "coop.ready" => {
                let lobby = self
                    .session
                    .as_ref()
                    .map(|n| n.seat == 1 && n.status == online::LOBBY)
                    .unwrap_or(false);
                if !lobby {
                    return Err(self.refusal("coop.ready_later"));
                }
                self.ready = !self.ready;
            }
            "coop.name" => self.save.name = online::clean_name(value),
            "coop.tab" => {
                if !online::PLACES.contains(&value) {
                    return Err(format!("There is no lobby tab \"{value}\"."));
                }
                self.lobby_tab = value.to_string();
            }
            "coop.more" => self.coop_more = true,
            "coop.by_invitation" => {
                self.room.clear();
                self.last_try.clear();
            }
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

    /// The save to store in this browser, when it differs from the one last
    /// stored (T25). Nothing mid-run: a run pays into the save at its end.
    pub fn take_autosave(&mut self) -> Option<String> {
        if self.screen == Screen::Run || self.stored.as_ref() == Some(&self.save) {
            return None;
        }
        self.stored = Some(self.save.clone());
        Some(self.save.to_json())
    }

    /// The save this browser kept, read when the page opens. An empty text
    /// is a first visit. A save that cannot be read leaves a new save in
    /// place and is returned so the page can set it aside rather than
    /// write over it; otherwise this returns nothing.
    pub fn restore(&mut self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        match Save::from_json(&self.content, text) {
            Ok(save) => {
                self.stored = Some(save.clone());
                self.save = save;
                String::new()
            }
            Err(_) => {
                self.notice = "save.stored_unreadable".into();
                text.to_string()
            }
        }
    }

    /// The page reports whether storing the save worked.
    pub fn storage_ok(&mut self, ok: bool) {
        self.store_ok = Some(ok);
        if !ok {
            // try again on the next change
            self.stored = None;
        }
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
    use std::collections::VecDeque;

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
    pub fn wire(a: &mut Game, b: &mut Game) {
        for (_, text, _) in a.net_outbox() {
            b.net_message(1, &text);
        }
        for (_, text, _) in b.net_outbox() {
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
        assert!(host.partner.is_some(), "the host heard the joiner's loadout");
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
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

    /// A host and a joiner, connected and in the lobby on the co-op screen.
    pub fn lobby() -> (Game, Game) {
        let mut host = game();
        let mut join = game();
        host.act("screen.show", "coop").unwrap();
        join.act("screen.show", "coop").unwrap();
        host.host(7, "gs");
        join.join("ABCDE", "gs").unwrap();
        host.net_peer(2);
        join.net_peer(1);
        wire(&mut host, &mut join);
        wire(&mut host, &mut join);
        (host, join)
    }

    #[test]
    fn gear_and_outfitter_changes_in_the_lobby_reach_the_host() {
        let (mut host, mut join) = lobby();
        let before = host.partner.clone().expect("the joiner's first seat").load.stats;
        // plating_rank: max_hp +10 a rank, 120 gold for the first
        join.save.gold = 120;
        join.act("shop.buy", "plating_rank").unwrap();
        // kinetic_core: damage_pct +3 at rank one, worn in the core slot
        join.save.modules.push(crate::meta::Piece {
            uid: 99,
            id: "kinetic_core".into(),
            rank: 1,
        });
        join.act("screen.show", "gear").unwrap();
        join.act("gear.equip", "99").unwrap();
        wire(&mut host, &mut join);
        let after = host.partner.clone().unwrap().load.stats;
        assert_eq!(after.max_hp, before.max_hp + 10, "the Outfitter rank arrived");
        assert_eq!(after.damage_pct, before.damage_pct + 3, "the worn module arrived");
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        let w = join.world.as_ref().unwrap();
        assert_eq!(
            w.players[1].load.stats.damage_pct,
            before.damage_pct + 3,
            "and the run has it"
        );
    }

    #[test]
    fn an_unchanged_loadout_is_not_sent_again() {
        let (_host, mut join) = lobby();
        join.act("coop.tab", "gear").unwrap();
        assert!(!join.net_outbox().is_empty(), "a new place is sent");
        join.act("coop.tab", "gear").unwrap();
        join.act("gear.pick", "0").unwrap_err();
        assert!(join.net_outbox().is_empty(), "nothing changed, nothing sent");
    }

    #[test]
    fn the_host_starts_only_once_the_partner_is_ready_and_their_own_change_takes_it_back() {
        let (mut host, mut join) = lobby();
        assert!(host.act("coop.start", "").is_err(), "not ready yet");
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
        assert!(host.partner.as_ref().unwrap().ready);
        // the partner buys an Outfitter rank: Ready is taken back
        join.save.gold = 120;
        join.act("shop.buy", "plating_rank").unwrap();
        wire(&mut host, &mut join);
        assert!(!join.ready && !host.partner.as_ref().unwrap().ready);
        assert!(host.act("coop.start", "").is_err());
        // the host's own change leaves the partner ready
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
        host.act("hangar.pick", "pathfinder").unwrap();
        host.save.gold = 120;
        host.act("shop.buy", "plating_rank").unwrap();
        wire(&mut host, &mut join);
        assert!(join.ready && host.partner.as_ref().unwrap().ready);
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        assert_eq!(join.screen, Screen::Run);
    }

    #[test]
    fn each_player_sees_the_other_s_name_character_and_place() {
        let (mut host, mut join) = lobby();
        host.act("coop.name", "  Sam   G ").unwrap();
        host.act("hangar.pick", "pathfinder").unwrap();
        join.act("coop.name", "Peter").unwrap();
        join.act("coop.tab", "gear").unwrap();
        wire(&mut host, &mut join);
        let seen_by_join = join.partner.clone().expect("the host's seat");
        assert_eq!(seen_by_join.name, "Sam G");
        assert_eq!(seen_by_join.load.character, "pathfinder");
        assert_eq!(
            seen_by_join.at, "character",
            "the host's lobby opens on Character"
        );
        let seen_by_host = host.partner.clone().unwrap();
        assert_eq!(
            (seen_by_host.name.as_str(), seen_by_host.at.as_str()),
            ("Peter", "gear")
        );
        assert!(join.act("coop.tab", "cellar").is_err());
    }

    #[test]
    fn leaving_sends_a_goodbye_then_closes_the_room() {
        let (mut host, mut join) = lobby();
        host.act("coop.leave", "").unwrap();
        assert!(!host.in_room(), "the screen shows the way in again at once");
        let out = host.net_outbox();
        assert!(out.iter().any(|(_, t, _)| t.contains("bye")), "{out:?}");
        assert!(host.session.is_none(), "the room is gone once the goodbye is out");
        for (_, text, _) in out {
            join.net_message(1, &text);
        }
        assert_eq!(join.session.as_ref().unwrap().status, online::LEFT_ROOM);
        assert!(host.act("run.start", "").is_ok(), "solo play is open again");
    }

    #[test]
    fn the_save_is_stored_once_after_each_change_and_never_mid_run() {
        let mut g = game();
        let first = g.take_autosave().expect("a first visit stores the new save");
        assert!(g.take_autosave().is_none(), "nothing changed");
        g.save.gold = 120;
        g.act("shop.buy", "plating_rank").unwrap();
        let bought = g.take_autosave().expect("a purchase is stored");
        assert_ne!(first, bought);
        assert!(bought.contains("\"plating_rank\": 1") || bought.contains("\"plating_rank\":1"));
        g.act("screen.show", "hangar").unwrap();
        g.act("run.start", "").unwrap();
        g.save.gold += 1;
        assert!(g.take_autosave().is_none(), "nothing is stored mid-run");
        g.act("run.quit", "").unwrap();
        let after = g.take_autosave().expect("the run's end is stored");
        assert!(
            after.contains("\"runs\": 1") || after.contains("\"runs\":1"),
            "the run counted"
        );
    }

    #[test]
    fn a_stored_save_comes_back_and_a_refused_store_is_tried_again() {
        let mut a = game();
        a.save.gold = 777;
        a.save.name = "Ana".into();
        let text = a.take_autosave().unwrap();
        let mut b = game();
        assert_eq!(b.restore(&text), "", "a good save needs no setting aside");
        assert_eq!((b.save.gold, b.save.name.as_str()), (777, "Ana"));
        assert!(
            b.take_autosave().is_none(),
            "what was just read is already stored"
        );
        assert_eq!(b.restore(""), "", "a first visit");
        // the browser refused the write: the next frame tries again
        b.save.gold = 1;
        assert!(b.take_autosave().is_some());
        b.storage_ok(false);
        assert!(b.take_autosave().is_some(), "tried again");
    }

    #[test]
    fn an_unreadable_stored_save_is_set_aside_for_a_new_one() {
        let mut g = game();
        let fresh = g.save.clone();
        let aside = g.restore("{ not a save");
        assert_eq!(aside, "{ not a save", "handed back to be kept, not written over");
        assert_eq!(g.save, fresh, "play goes on with a new save");
        assert_eq!(g.notice, "save.stored_unreadable");
    }

    /// Every way out of co-op keeps what the run paid (the save lives in
    /// the page until it is downloaded; nothing here clears it).
    #[test]
    fn leaving_co_op_keeps_your_progress() {
        let runs = |g: &Game| g.save.stats.get("coop_runs").copied().unwrap_or(0);
        // quitting mid-run, and the partner who sees them go
        let (mut host, mut join) = lobby();
        host.save.gold = 500;
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        for f in 0..300i64 {
            host.advance(f * 16_667, Raw::default());
            join.advance(f * 16_667, Raw::default());
            wire(&mut host, &mut join);
        }
        join.act("run.quit", "").unwrap();
        wire(&mut host, &mut join);
        host.advance(301 * 16_667, Raw::default());
        for (who, g) in [("the one who quit", &join), ("the one left behind", &host)] {
            assert_eq!(g.screen, Screen::Results, "{who}");
            assert_eq!(runs(g), 1, "{who}: the run counted");
            let paid = g.payout.as_ref().expect("a payout").gold;
            assert!(g.save.gold >= paid, "{who}: the gold reached the save");
        }
        assert!(host.save.gold >= 500, "gold held before the run stays");
        // leaving from the lobby touches nothing in the save
        let before = host.save.clone();
        host.act("screen.show", "coop").unwrap();
        host.act("coop.leave", "").unwrap();
        host.net_outbox();
        assert_eq!(host.save, before);
    }

    #[test]
    fn solo_play_waits_until_you_leave_the_room() {
        let (mut host, _join) = lobby();
        assert!(host.act("run.start", "").is_err());
    }

    #[test]
    fn after_a_co_op_run_both_are_back_in_the_lobby_and_changes_are_sent() {
        let (mut host, mut join) = lobby();
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        host.end_run();
        join.end_run();
        wire(&mut host, &mut join);
        for g in [&host, &join] {
            assert_eq!(g.session.as_ref().unwrap().status, online::LOBBY);
        }
        join.save.gold = 120;
        let before = host.partner.clone().unwrap().load.stats.max_hp;
        join.act("shop.buy", "plating_rank").unwrap();
        wire(&mut host, &mut join);
        assert_eq!(host.partner.clone().unwrap().load.stats.max_hp, before + 10);
    }

    /// Two co-op games whose messages take `lag` frames, losing every
    /// `lose`-th fast message, for `frames` frames.
    fn lagged_coop(lag: usize, lose: usize, frames: i64, host_bits: fn(i64) -> u32) -> (Game, Game) {
        let mut host = game();
        let mut join = game();
        host.host(7, "gs");
        join.join("ABCDE", "gs").unwrap();
        host.net_peer(2);
        join.net_peer(1);
        wire(&mut host, &mut join);
        wire(&mut host, &mut join);
        join.act("coop.ready", "").unwrap();
        wire(&mut host, &mut join);
        host.act("coop.start", "").unwrap();
        wire(&mut host, &mut join);
        let mut to_join: VecDeque<Vec<(u32, String, bool)>> = VecDeque::new();
        let mut to_host: VecDeque<Vec<(u32, String, bool)>> = VecDeque::new();
        let mut n = [0usize; 2];
        for f in 0..frames {
            let raw = Raw {
                bits: host_bits(f),
                ..Raw::default()
            };
            host.advance(f * 16_667, raw);
            join.advance(
                f * 16_667 + 3_000,
                Raw {
                    bits: if (f / 25) % 2 == 0 { 4 } else { 1 },
                    ..Raw::default()
                },
            );
            for g in [&mut host, &mut join] {
                if g.world
                    .as_ref()
                    .map(|w| w.players[g.seat].offer.is_some())
                    .unwrap_or(false)
                {
                    g.act("card.take", "1").unwrap();
                }
            }
            let mut drop = |k: usize, out: Vec<(u32, String, bool)>| -> Vec<(u32, String, bool)> {
                out.into_iter()
                    .filter(|(_, _, fast)| {
                        if !*fast {
                            return true;
                        }
                        n[k] += 1;
                        lose == 0 || !n[k].is_multiple_of(lose)
                    })
                    .collect()
            };
            to_join.push_back(drop(0, host.net_outbox()));
            to_host.push_back(drop(1, join.net_outbox()));
            if to_join.len() > lag {
                for (_, t, _) in to_join.pop_front().unwrap() {
                    join.net_message(1, &t);
                }
                for (_, t, _) in to_host.pop_front().unwrap() {
                    host.net_message(2, &t);
                }
            }
        }
        (host, join)
    }

    #[test]
    fn a_laggy_lossy_co_op_run_stays_in_step_and_keeps_moving() {
        // 6 frames each way is a 200 ms round trip; one fast message in four lost
        let (host, join) = lagged_coop(6, 4, 1500, |f| if (f / 40) % 2 == 0 { 8 } else { 2 });
        for g in [&host, &join] {
            let n = g.session.as_ref().unwrap();
            assert_eq!(n.status, online::PLAYING, "the checksums agree");
            assert!(n.stats.rollbacks > 0, "a 200 ms trip needs corrections");
            // the lockstep this replaced stood still for every late input;
            // rollback waits only past MAX_AHEAD
            assert!(n.stats.stalls < 300, "waited {} frames of 1500", n.stats.stalls);
            assert!(n.stats.rtt_us >= 180_000, "ping measured: {}", n.stats.rtt_us);
        }
        let t = host.world.as_ref().unwrap().tick;
        assert!(t > 1100, "the run kept moving: tick {t} after 1500 frames");
    }

    #[test]
    fn in_co_op_your_own_move_shows_within_three_ticks() {
        // standing still for two seconds, then left, over a 200 ms round trip
        let still = lagged_coop(6, 0, 120, |_| 0).0;
        let (host, _join) = lagged_coop(6, 0, 124, |f| if f < 120 { 0 } else { 4 });
        let (a, b) = (still.world.as_ref().unwrap(), host.world.as_ref().unwrap());
        assert!(
            b.players[0].pos.x < a.players[0].pos.x,
            "own input showed within four frames (DELAY {})",
            online::DELAY
        );
    }

    #[test]
    fn the_rig_window_moves_a_part_by_two_clicks_while_the_run_goes_on() {
        let mut g = game();
        g.act("run.start", "").unwrap();
        g.world.as_mut().unwrap().players[0].spare.push("ignite".into());
        g.act("rig.open", "").unwrap();
        let ui = crate::view::build(&g);
        let overlay = ui.overlay.expect("the rig window");
        assert_eq!(overlay.id, "rigs");
        assert!(overlay
            .items
            .iter()
            .any(|i| i.id == "rig-x0" && i.title == g.text.name("ignite")));
        // pick up the spare Ignite, put it before the Seeker Rifle's socket
        g.act("rig.pick", "x0").unwrap();
        assert!(g.rig_pick.is_some());
        g.act("rig.pick", "r0s1").unwrap();
        assert!(g.rig_pick.is_none());
        let t = g.world.as_ref().unwrap().tick;
        for f in 0..4 {
            g.advance(f * 16_667, Raw::default());
        }
        let w = g.world.as_ref().unwrap();
        assert!(w.tick > t, "the run kept going with the window open");
        assert_eq!(w.players[0].rigs[0].sockets[1], "ignite");
        assert!(w.players[0].spare.is_empty());
        // clicking an empty place picks nothing up
        g.act("rig.pick", "r1s0").unwrap();
        assert!(g.rig_pick.is_none());
        assert!(g.act("rig.pick", "nowhere").is_err());
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
        join.act("coop.ready", "").unwrap();
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
