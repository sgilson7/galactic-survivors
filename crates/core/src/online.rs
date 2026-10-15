//! Online co-op, core side: what crosses the wire and the bookkeeping each
//! browser keeps. From the online kit (`kits/online/README.md`), with the
//! kit's toy world replaced by the run.
//!
//! The transport in `web/net/rtc.js` connects two browsers and carries
//! text. This module answers each event by leaving text in an outbox. Both
//! browsers run the same `World` from the same seed and both loadouts.
//!
//! Rollback, after Raiders of the Lost Mart's `rollback.rs` (T13). The
//! first build waited for the partner's input before every tick, used each
//! key six ticks late, and froze both screens whenever a message ran late.
//! Now:
//!
//! * a browser's own input is used `DELAY` ticks after it is made;
//! * the partner is assumed to keep doing what they last did (moving and
//!   aiming; never a one-shot action such as a card);
//! * when the partner's true input differs from that guess for a tick
//!   already run, the world goes back to the copy kept before that tick and
//!   the ticks since run again;
//! * no browser runs more than `MAX_AHEAD` ticks past the last tick it holds
//!   both inputs for; there it waits.
//!
//! Inputs travel on an unordered channel that never resends. Each message
//! carries every input the partner has not acknowledged, so a lost message
//! is covered by the next one instead of holding the rest back. Everything
//! else goes on the reliable channel. Every `SUM_EVERY` ticks each browser
//! checksums a confirmed tick, and a mismatch is reported.
//!
//! Every status is a key for a string in `data/strings.json`.

use crate::world::Loadout;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// One movement and aim with no one-shot action: what a guess repeats.
pub const HELD_MASK: u64 = 0xff_ffff_ffff;

/// Bumped whenever a message changes shape or a rule changes how a tick
/// runs. `web/net/config.js` carries the same number (online_fence.rs).
pub const PROTOCOL: u32 = 2;

/// Ticks between a local input and the tick that uses it: 33 ms at 60
/// ticks a second. It hides part of the trip, so short trips need no
/// correction at all.
pub const DELAY: u32 = 2;

/// The furthest a browser runs past its confirmed ticks: 133 ms. Raiders
/// uses 14; a late-run tick here costs more (MEASUREMENTS.md, "Cost of one
/// tick, for rollback"), so a re-run is kept short enough for one frame.
pub const MAX_AHEAD: u32 = 8;

/// The most inputs one message carries.
pub const MAX_BATCH: usize = 64;

/// Frames between pings.
pub const PING_EVERY: u32 = 30;

/// Ticks between checksums.
pub const SUM_EVERY: u32 = 30;

pub const SEATS: usize = 2;

pub const WAITING: &str = "online.waiting";
pub const LOBBY: &str = "online.lobby";
pub const PLAYING: &str = "online.playing";
pub const RELOAD: &str = "online.reload";
pub const GARBLED: &str = "online.garbled";
pub const LEFT_ROOM: &str = "online.left";
pub const FULL: &str = "online.full";
pub const DESYNC: &str = "online.desync";

const REASONS: &[&str] = &[RELOAD, FULL];

pub type Peer = u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Msg {
    Hello {
        protocol: u32,
        build: String,
    },
    /// The joiner's loadout, sent after the hello and again when it changes.
    Load {
        load: Loadout,
    },
    /// The host starts the run: the seed and both loadouts, host first.
    Start {
        seed: u64,
        loads: Vec<Loadout>,
    },
    /// The sender's inputs from tick `from` on. `ack`: every tick below it
    /// holds the receiver's input at the sender. `at`: the sender's next tick.
    In {
        from: u32,
        keys: Vec<u64>,
        ack: u32,
        at: u32,
    },
    /// A ping and its echo, in the sender's microseconds.
    Ping {
        us: i64,
    },
    Pong {
        us: i64,
    },
    Sum {
        tick: u32,
        sum: u64,
    },
    Bye {
        why: String,
    },
}

/// Whether a message goes on the fast channel, which may drop or reorder it.
pub fn fast(m: &Msg) -> bool {
    matches!(m, Msg::In { .. } | Msg::Ping { .. } | Msg::Pong { .. })
}

pub fn encode(m: &Msg) -> String {
    serde_json::to_string(m).expect("a Msg always serialises")
}

pub fn decode(text: &str) -> Option<Msg> {
    serde_json::from_str(text).ok()
}

/// The room name the transport joins: the app id, the protocol and the code
/// the players typed. No build hash, so a stale tab still meets a fresh one
/// and is told to reload.
pub fn room_name(app_id: &str, code: &str) -> String {
    format!("{app_id}-p{PROTOCOL}-{code}")
}

/// Turns a typed room code into its canonical form, or None if it cannot
/// be one: four to eight letters or digits.
pub fn clean_code(code: &str) -> Option<String> {
    let c: String = code
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if (4..=8).contains(&c.len()) && c.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(c)
    } else {
        None
    }
}

/// A five-letter room code from a number the page drew. Letters that are
/// easy to confuse (I, O, 0, 1) are left out.
pub fn make_code(mut n: u64) -> String {
    const LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut s = String::new();
    for _ in 0..5 {
        s.push(LETTERS[(n % LETTERS.len() as u64) as usize] as char);
        n /= LETTERS.len() as u64;
    }
    s
}

/// What a browser learned from a message that the game must act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    PeerLoad(Loadout),
    Start { seed: u64, loads: Vec<Loadout> },
}

/// What the connection readout shows (T13): the last round trip, and how
/// often this browser waited or ran ticks again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// The round trip in microseconds, smoothed; 0 before the first echo.
    pub rtt_us: i64,
    pub rollbacks: u32,
    pub rolled_ticks: u32,
    /// Frames on which this browser had to wait for the partner.
    pub stalls: u32,
}

#[derive(Debug)]
pub struct Session<W: Clone = crate::world::World> {
    pub seat: usize,
    pub status: &'static str,
    build: String,
    pub peer: Option<Peer>,
    greeting: Option<Peer>,
    pub running: bool,
    outbox: VecDeque<(Peer, String, bool)>,
    events: VecDeque<Event>,
    /// The next tick to run.
    pub current: u32,
    /// Inputs heard (or made) per seat, by tick.
    known: [BTreeMap<u32, u64>; SEATS],
    /// Per seat: every tick below this is known.
    upto: [u32; SEATS],
    /// Every tick below this holds this browser's input at the partner.
    acked: u32,
    /// The partner's next tick, as it last said.
    their_at: u32,
    /// The inputs each run tick used, while one may yet be wrong.
    used: VecDeque<(u32, [u64; SEATS])>,
    /// The world before each tick that may need running again.
    snaps: VecDeque<(u32, W)>,
    rollback_to: Option<u32>,
    sums: BTreeMap<u32, u64>,
    /// Checksums from the partner for ticks this browser has not confirmed.
    their_sums: BTreeMap<u32, u64>,
    pings: u32,
    pub stats: Stats,
}

impl<W: Clone> Session<W> {
    pub fn new(seat: usize, build: &str) -> Session<W> {
        Session {
            seat,
            status: WAITING,
            build: build.to_string(),
            peer: None,
            greeting: None,
            running: false,
            outbox: VecDeque::new(),
            events: VecDeque::new(),
            current: 0,
            known: Default::default(),
            upto: [0; SEATS],
            acked: 0,
            their_at: 0,
            used: VecDeque::new(),
            snaps: VecDeque::new(),
            rollback_to: None,
            sums: BTreeMap::new(),
            their_sums: BTreeMap::new(),
            pings: 0,
            stats: Stats::default(),
        }
    }

    fn send(&mut self, to: Peer, m: &Msg) {
        self.outbox.push_back((to, encode(m), fast(m)));
    }

    /// Everything to send: the peer, the text, and whether it may go on
    /// the fast channel.
    pub fn take_outbox(&mut self) -> Vec<(Peer, String, bool)> {
        self.outbox.drain(..).collect()
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }

    pub fn connected(&mut self, peer: Peer) {
        if self.peer.is_some() || self.greeting.is_some() {
            self.send(
                peer,
                &Msg::Bye {
                    why: FULL.to_string(),
                },
            );
            return;
        }
        self.greeting = Some(peer);
        let hello = Msg::Hello {
            protocol: PROTOCOL,
            build: self.build.clone(),
        };
        self.send(peer, &hello);
    }

    pub fn left(&mut self, peer: Peer) {
        if self.peer == Some(peer) || self.greeting == Some(peer) {
            self.peer = None;
            self.greeting = None;
            self.status = LEFT_ROOM;
            self.running = false;
        }
    }

    /// Sends this browser's loadout to the other (the joiner does this).
    pub fn send_load(&mut self, load: &Loadout) {
        if let Some(peer) = self.peer {
            self.send(peer, &Msg::Load { load: load.clone() });
        }
    }

    /// The host starts the run; both browsers begin at tick 0.
    pub fn start(&mut self, seed: u64, loads: Vec<Loadout>) {
        let Some(peer) = self.peer else { return };
        self.send(peer, &Msg::Start { seed, loads });
        self.begin();
    }

    fn begin(&mut self) {
        self.current = 0;
        self.running = true;
        self.acked = 0;
        self.their_at = 0;
        self.used.clear();
        self.snaps.clear();
        self.rollback_to = None;
        self.sums.clear();
        self.their_sums.clear();
        self.stats = Stats::default();
        // the first ticks have no input from anybody: the same on both
        for seat in 0..SEATS {
            self.known[seat].clear();
            for t in 0..DELAY {
                self.known[seat].insert(t, 0);
            }
            self.upto[seat] = DELAY;
        }
        self.status = PLAYING;
    }

    /// Every tick below this has both seats' true inputs.
    pub fn confirmed(&self) -> u32 {
        self.upto[0].min(self.upto[1])
    }

    /// How far this browser runs ahead of the partner, in ticks.
    pub fn ahead(&self) -> i32 {
        self.current as i32 - self.their_at as i32
    }

    pub fn message(&mut self, from: Peer, text: &str, now_us: i64) {
        let known = self.peer == Some(from) || self.greeting == Some(from);
        if !known {
            return;
        }
        let Some(msg) = decode(text) else {
            self.status = GARBLED;
            return;
        };
        match msg {
            Msg::Hello { protocol, build } => {
                if protocol != PROTOCOL || build != self.build {
                    self.send(
                        from,
                        &Msg::Bye {
                            why: RELOAD.to_string(),
                        },
                    );
                    self.greeting = None;
                    self.peer = None;
                    self.status = RELOAD;
                } else {
                    self.greeting = None;
                    self.peer = Some(from);
                    self.status = LOBBY;
                }
            }
            Msg::Bye { why } => {
                self.status = REASONS.iter().copied().find(|r| *r == why).unwrap_or(LEFT_ROOM);
                self.peer = None;
                self.greeting = None;
                self.running = false;
            }
            _ if self.peer != Some(from) => {
                self.status = GARBLED;
            }
            Msg::Load { load } => {
                if self.seat == 0 {
                    self.events.push_back(Event::PeerLoad(load));
                } else {
                    self.status = GARBLED;
                }
            }
            Msg::Start { seed, loads } => {
                if self.seat == 1 && loads.len() == SEATS {
                    self.begin();
                    self.events.push_back(Event::Start { seed, loads });
                } else {
                    self.status = GARBLED;
                }
            }
            Msg::Ping { us } => self.send(from, &Msg::Pong { us }),
            Msg::Pong { us } => {
                let rtt = (now_us - us).max(0);
                self.stats.rtt_us = if self.stats.rtt_us == 0 {
                    rtt
                } else {
                    (self.stats.rtt_us * 3 + rtt) / 4
                };
            }
            // inputs still in flight when a run ends are not an error
            Msg::In { .. } | Msg::Sum { .. } if !self.running => {}
            Msg::In {
                from: start,
                keys,
                ack,
                at,
            } => {
                let other = 1 - self.seat;
                // the partner can be at most a window ahead of this browser
                let limit = self.current + DELAY * 2 + MAX_AHEAD + MAX_BATCH as u32;
                if keys.len() > MAX_BATCH || start as u64 + keys.len() as u64 > limit as u64 {
                    self.status = GARBLED;
                    return;
                }
                self.acked = self.acked.max(ack.min(self.upto[self.seat]));
                self.their_at = self.their_at.max(at);
                for (k, v) in keys.iter().enumerate() {
                    self.record(other, start + k as u32, *v);
                }
            }
            Msg::Sum { tick, sum } => match self.sums.get(&tick) {
                Some(mine) if *mine != sum => self.status = DESYNC,
                Some(_) => {}
                None => {
                    self.their_sums.insert(tick, sum);
                }
            },
        }
    }

    fn record(&mut self, seat: usize, t: u32, keys: u64) {
        if t < self.upto[seat] || self.known[seat].contains_key(&t) {
            return;
        }
        self.known[seat].insert(t, keys);
        while self.known[seat].contains_key(&self.upto[seat]) {
            self.upto[seat] += 1;
        }
        // was a different input used for this tick?
        if t < self.current {
            if let Some((_, used)) = self.used.iter().find(|(ut, _)| *ut == t) {
                if used[seat] != keys {
                    self.rollback_to = Some(self.rollback_to.map_or(t, |r| r.min(t)));
                }
            }
        }
    }

    /// The input a seat gives on a tick, as well as it is known: the true
    /// one, or the last one heard with its one-shot actions removed.
    pub fn input_for(&self, seat: usize, t: u32) -> u64 {
        if let Some(k) = self.known[seat].get(&t) {
            return *k;
        }
        self.known[seat]
            .range(..t)
            .next_back()
            .map(|(_, k)| *k & HELD_MASK)
            .unwrap_or(0)
    }

    /// This browser's input, made now, for every tick up to `DELAY` ahead
    /// that has none yet. `once` carries the one-shot actions, which only
    /// the first tick filled takes.
    pub fn local(&mut self, held: u64, once: u64) {
        if self.peer.is_none() || !self.running {
            return;
        }
        let me = self.seat;
        let to = self.current + DELAY;
        let mut extra = once;
        while self.upto[me] <= to {
            let t = self.upto[me];
            self.record(me, t, held | extra);
            extra = 0;
        }
    }

    /// The inputs the partner has not acknowledged, with this browser's
    /// acknowledgement of theirs.
    pub fn send_inputs(&mut self) {
        let Some(peer) = self.peer else { return };
        if !self.running {
            return;
        }
        let me = self.seat;
        let from = self.acked.max(self.upto[me].saturating_sub(MAX_BATCH as u32));
        let keys: Vec<u64> = (from..self.upto[me])
            .map(|t| self.known[me].get(&t).copied().unwrap_or(0))
            .collect();
        let m = Msg::In {
            from,
            keys,
            ack: self.upto[1 - me],
            at: self.current,
        };
        self.send(peer, &m);
    }

    /// The same, on the reliable channel: the last word before a run ends,
    /// so the partner can confirm the final ticks.
    pub fn send_inputs_reliably(&mut self) {
        self.send_inputs();
        if let Some((_, _, fast)) = self.outbox.back_mut() {
            *fast = false;
        }
    }

    /// A ping now and then, timed in the page's microseconds.
    pub fn maybe_ping(&mut self, now_us: i64) {
        let Some(peer) = self.peer else { return };
        self.pings += 1;
        if self.pings.is_multiple_of(PING_EVERY) {
            self.send(peer, &Msg::Ping { us: now_us });
        }
    }

    /// Runs up to `due` new ticks (after any rollback). `step` runs one
    /// tick with both seats' inputs, and `sum` checksums a world.
    pub fn advance(
        &mut self,
        world: &mut W,
        due: u32,
        step: &mut dyn FnMut(&mut W, [u64; SEATS]),
        sum: &dyn Fn(&W) -> u64,
    ) {
        if let Some(to) = self.rollback_to.take() {
            if let Some(k) = self.snaps.iter().position(|(t, _)| *t == to) {
                *world = self.snaps[k].1.clone();
                self.snaps.truncate(k);
                self.used.retain(|(t, _)| *t < to);
                self.stats.rollbacks += 1;
                let target = self.current;
                self.current = to;
                while self.current < target {
                    self.stats.rolled_ticks += 1;
                    self.tick_once(world, step, sum);
                }
            }
        }
        for _ in 0..due {
            if self.current >= self.confirmed() + MAX_AHEAD {
                self.stats.stalls += 1;
                break;
            }
            self.tick_once(world, step, sum);
        }
        // what is confirmed can never be rolled back past. A kept copy of
        // the world before a confirmed tick is final, so a tick that first
        // ran on a guess is checksummed here once its inputs are known.
        let c = self.confirmed();
        let due_sums: Vec<(u32, u64)> = self
            .snaps
            .iter()
            .filter(|(t, _)| *t <= c && *t > 0 && t.is_multiple_of(SUM_EVERY) && !self.sums.contains_key(t))
            .map(|(t, w)| (*t, sum(w)))
            .collect();
        for (t, s) in due_sums {
            self.check_sum(t, s);
        }
        while self.snaps.len() > 1 && self.snaps.get(1).is_some_and(|(t, _)| *t <= c) {
            self.snaps.pop_front();
        }
        while self.used.front().is_some_and(|(t, _)| *t < c) {
            self.used.pop_front();
        }
        self.prune();
    }

    fn tick_once(
        &mut self,
        world: &mut W,
        step: &mut dyn FnMut(&mut W, [u64; SEATS]),
        sum: &dyn Fn(&W) -> u64,
    ) {
        let t = self.current;
        let inputs = [self.input_for(0, t), self.input_for(1, t)];
        let confirmed = t < self.confirmed();
        if !confirmed {
            self.snaps.push_back((t, world.clone()));
            self.used.push_back((t, inputs));
        }
        step(world, inputs);
        self.current = t + 1;
        // a confirmed tick's world is final: checksum it now and then
        if confirmed && self.current.is_multiple_of(SUM_EVERY) && !self.sums.contains_key(&self.current) {
            let s = sum(world);
            self.check_sum(self.current, s);
        }
    }

    fn check_sum(&mut self, tick: u32, s: u64) {
        if let Some(theirs) = self.their_sums.remove(&tick) {
            if theirs != s {
                self.status = DESYNC;
            }
        }
        self.sums.insert(tick, s);
        while self.sums.len() > 16 {
            let first = *self.sums.keys().next().unwrap_or(&0);
            self.sums.remove(&first);
        }
        if let Some(peer) = self.peer {
            self.send(peer, &Msg::Sum { tick, sum: s });
        }
    }

    /// Drops inputs that will never be needed again.
    fn prune(&mut self) {
        let keep = self
            .confirmed()
            .saturating_sub(4)
            .min(self.acked.saturating_sub(4));
        for seat in 0..SEATS {
            while let Some((&t, _)) = self.known[seat].iter().next() {
                // keep the newest known input for guesses
                if t >= keep || self.known[seat].len() <= 1 {
                    break;
                }
                self.known[seat].remove(&t);
            }
        }
    }

    pub fn leave(&mut self) {
        if let Some(peer) = self.peer.or(self.greeting) {
            self.send(
                peer,
                &Msg::Bye {
                    why: LEFT_ROOM.to_string(),
                },
            );
        }
        self.peer = None;
        self.greeting = None;
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: Peer = 1;
    const JOINER: Peer = 2;

    /// A toy world for the session tests: two counters moved by the keys,
    /// and the inputs each tick used.
    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct Toy {
        pos: [i64; SEATS],
        log: Vec<[u64; SEATS]>,
    }

    impl Toy {
        fn step(&mut self, keys: [u64; SEATS]) {
            for (s, k) in keys.iter().enumerate() {
                self.pos[s] += (*k & 0xff) as i64 % 3 - 1;
            }
            self.log.push(keys);
        }
        fn sum(&self) -> u64 {
            (self.pos[0] as u64).wrapping_mul(31) ^ (self.pos[1] as u64) ^ ((self.log.len() as u64) << 40)
        }
    }

    type S = Session<Toy>;

    fn deliver(to: &mut S, from: Peer, out: Vec<(Peer, String, bool)>) {
        for (_, text, _) in out {
            to.message(from, &text, 0);
        }
    }

    fn pair(build_host: &str, build_join: &str) -> (S, S) {
        let mut host = S::new(0, build_host);
        let mut join = S::new(1, build_join);
        host.connected(JOINER);
        join.connected(HOST);
        let out = host.take_outbox();
        deliver(&mut join, HOST, out);
        let out = join.take_outbox();
        deliver(&mut host, JOINER, out);
        (host, join)
    }

    fn started() -> (S, S) {
        let (mut host, mut join) = pair("b", "b");
        host.start(9, vec![Loadout::default(), Loadout::default()]);
        let out = host.take_outbox();
        deliver(&mut join, HOST, out);
        (host, join)
    }

    /// One display frame on one browser: its input, its ticks, its message.
    fn frame(s: &mut S, w: &mut Toy, held: u64, once: u64, due: u32) {
        s.local(held, once);
        s.advance(w, due, &mut |w: &mut Toy, k| w.step(k), &|w: &Toy| w.sum());
        s.send_inputs();
    }

    fn script(seat: usize, f: u32) -> u64 {
        // held keys change every few frames, as a player's do
        ((f / (5 + seat as u32 * 3)) % 7) as u64
    }

    #[test]
    fn every_message_round_trips() {
        let all = [
            Msg::Hello {
                protocol: PROTOCOL,
                build: "abc".into(),
            },
            Msg::Load {
                load: Loadout {
                    character: "vector".into(),
                    ..Loadout::default()
                },
            },
            Msg::Start {
                seed: u64::MAX,
                loads: vec![Loadout::default(), Loadout::default()],
            },
            Msg::In {
                from: 7,
                keys: vec![0xffff_ffff_ffff_ffff, 0],
                ack: 3,
                at: 9,
            },
            Msg::Ping { us: -5 },
            Msg::Pong { us: 1 << 40 },
            Msg::Sum { tick: 30, sum: 5 },
            Msg::Bye { why: FULL.into() },
        ];
        for m in all {
            assert_eq!(decode(&encode(&m)), Some(m.clone()), "{m:?}");
        }
    }

    #[test]
    fn only_inputs_and_pings_take_the_fast_channel() {
        assert!(fast(&Msg::In {
            from: 0,
            keys: vec![],
            ack: 0,
            at: 0
        }));
        assert!(fast(&Msg::Ping { us: 0 }));
        assert!(!fast(&Msg::Sum { tick: 30, sum: 1 }));
        assert!(!fast(&Msg::Start {
            seed: 1,
            loads: vec![]
        }));
    }

    #[test]
    fn the_room_name_carries_the_protocol_and_not_the_build() {
        assert_eq!(room_name("gs", "ABCDE"), format!("gs-p{PROTOCOL}-ABCDE"));
        assert_eq!(clean_code(" abcd e"), Some("ABCDE".into()));
        assert_eq!(clean_code("ab"), None);
        assert_eq!(make_code(0), "AAAAA");
    }

    #[test]
    fn another_build_or_protocol_is_turned_away_with_a_reason() {
        let (host, join) = pair("b1", "b2");
        assert_eq!(host.status, RELOAD);
        assert_eq!(join.status, RELOAD);
        assert!(host.peer.is_none() && join.peer.is_none());
    }

    #[test]
    fn garbage_on_the_wire_changes_only_the_status() {
        let (mut host, _) = started();
        let before = host.current;
        for junk in [
            "",
            "{",
            "{\"t\":\"nope\"}",
            "{\"t\":\"in\",\"from\":99999,\"keys\":[1],\"ack\":0,\"at\":0}",
        ] {
            host.status = PLAYING;
            host.message(JOINER, junk, 0);
            assert_eq!(host.status, GARBLED, "{junk}");
            assert_eq!(host.current, before);
        }
        // a stranger is ignored outright
        host.status = PLAYING;
        host.message(77, "{", 0);
        assert_eq!(host.status, PLAYING);
    }

    #[test]
    fn a_browser_uses_its_own_input_two_ticks_later_without_waiting() {
        let (mut host, _) = started();
        let mut w = Toy::default();
        // nothing ever arrives from the joiner
        frame(&mut host, &mut w, 2, 0, 1);
        frame(&mut host, &mut w, 2, 0, 1);
        frame(&mut host, &mut w, 2, 0, 1);
        assert_eq!(w.log.len(), 3, "a browser without the partner's input still runs");
        assert_eq!(w.log[0][0], 0);
        assert_eq!(w.log[1][0], 0);
        assert_eq!(
            w.log[DELAY as usize][0], 2,
            "own input lands DELAY ticks after it is made"
        );
    }

    #[test]
    fn a_browser_runs_at_most_max_ahead_ticks_on_guesses() {
        let (mut host, _) = started();
        let mut w = Toy::default();
        for _ in 0..100 {
            frame(&mut host, &mut w, 1, 0, 1);
        }
        // the joiner's first DELAY ticks are known (empty); after that, guesses
        assert_eq!(host.current, DELAY + MAX_AHEAD);
        assert_eq!(host.stats.stalls, 100 - (DELAY + MAX_AHEAD));
    }

    #[test]
    fn a_guess_repeats_movement_but_never_a_one_shot_action() {
        let (mut host, _) = started();
        let card: u64 = 3 << 40;
        let rig: u64 = 0x0123 << 48;
        host.record(1, DELAY, 0x22 | card | rig);
        assert_eq!(host.input_for(1, DELAY), 0x22 | card | rig);
        assert_eq!(
            host.input_for(1, DELAY + 1),
            0x22,
            "the guess drops the card and the rig change"
        );
    }

    #[test]
    fn a_one_shot_action_rides_on_one_tick_only() {
        let (mut host, _) = started();
        let mut w = Toy::default();
        // a slow frame: three ticks run before this browser's next input
        host.advance(&mut w, 3, &mut |w: &mut Toy, k| w.step(k), &|w: &Toy| w.sum());
        let card: u64 = 1 << 40;
        // ticks 2 to 5 have no input of this browser's yet: four are filled
        host.local(5, card);
        assert_eq!(host.input_for(0, DELAY), 5 | card);
        for t in DELAY + 1..=3 + DELAY {
            assert_eq!(host.input_for(0, t), 5, "tick {t} carries no card");
        }
    }

    #[test]
    fn a_wrong_guess_is_rolled_back_and_both_copies_agree_on_every_confirmed_tick() {
        // lag in frames, and how often a fast message is lost
        for (lag, lose_every) in [(0usize, 0usize), (2, 0), (5, 3), (9, 2), (3, 5)] {
            let (mut host, mut join) = started();
            let (mut wh, mut wj) = (Toy::default(), Toy::default());
            let mut wire_hj: VecDeque<Vec<(Peer, String, bool)>> = VecDeque::new();
            let mut wire_jh: VecDeque<Vec<(Peer, String, bool)>> = VecDeque::new();
            // each direction loses its own every `lose_every`-th fast message
            let mut sent = [0usize; 2];
            let mut lossy = |dir: usize, out: Vec<(Peer, String, bool)>| -> Vec<(Peer, String, bool)> {
                out.into_iter()
                    .filter(|(_, _, f)| {
                        if !*f {
                            return true;
                        }
                        sent[dir] += 1;
                        !(lose_every > 0 && sent[dir].is_multiple_of(lose_every))
                    })
                    .collect()
            };
            for f in 0..900u32 {
                frame(&mut host, &mut wh, script(0, f), 0, 1);
                frame(&mut join, &mut wj, script(1, f), 0, 1);
                wire_hj.push_back(lossy(0, host.take_outbox()));
                wire_jh.push_back(lossy(1, join.take_outbox()));
                if wire_hj.len() > lag {
                    // the later message first: the fast channel may reorder
                    let mut a = wire_hj.pop_front().unwrap();
                    a.reverse();
                    deliver(&mut join, HOST, a);
                    let mut b = wire_jh.pop_front().unwrap();
                    b.reverse();
                    deliver(&mut host, JOINER, b);
                }
            }
            // the last messages may call for a rollback, which runs on the
            // next frame
            frame(&mut host, &mut wh, 0, 0, 0);
            frame(&mut join, &mut wj, 0, 0, 0);
            // inputs can be known for ticks not yet run
            let c = (host.confirmed().min(join.confirmed()) as usize)
                .min(wh.log.len())
                .min(wj.log.len());
            assert!(c > 600, "lag {lag}: only {c} ticks confirmed");
            let first = (0..c).find(|&t| wh.log[t] != wj.log[t]);
            assert_eq!(first, None, "lag {lag}: the copies used different inputs from tick {first:?} ({:?} against {:?}); confirmed {c}", first.map(|t| wh.log[t]), first.map(|t| wj.log[t]));
            if lag > 0 {
                assert!(
                    host.stats.rollbacks + join.stats.rollbacks > 0,
                    "lag {lag}: never rolled back"
                );
                // ticks first run on a guess are checksummed once confirmed
                assert!(
                    host.sums.len() >= 3,
                    "lag {lag}: only {} checksums",
                    host.sums.len()
                );
            }
            assert_eq!(host.status, PLAYING, "lag {lag}");
            assert_eq!(join.status, PLAYING, "lag {lag}");
            // and each copy's own seat is what it pressed, not a guess
            for t in 0..c {
                assert_eq!(wh.log[t], wj.log[t]);
            }
        }
    }

    #[test]
    fn worlds_that_disagree_are_reported_as_a_desync() {
        let (mut host, mut join) = started();
        let (mut wh, mut wj) = (Toy::default(), Toy::default());
        // the joiner's world counts differently from tick 0
        wj.pos[0] = 1000;
        for f in 0..120u32 {
            frame(&mut host, &mut wh, script(0, f), 0, 1);
            frame(&mut join, &mut wj, script(1, f), 0, 1);
            let out = host.take_outbox();
            deliver(&mut join, HOST, out);
            let out = join.take_outbox();
            deliver(&mut host, JOINER, out);
        }
        assert_eq!(host.status, DESYNC);
    }

    #[test]
    fn copies_that_agree_exchange_checksums_without_complaint() {
        let (mut host, mut join) = started();
        let (mut wh, mut wj) = (Toy::default(), Toy::default());
        for f in 0..300u32 {
            frame(&mut host, &mut wh, script(0, f), 0, 1);
            frame(&mut join, &mut wj, script(1, f), 0, 1);
            let out = host.take_outbox();
            deliver(&mut join, HOST, out);
            let out = join.take_outbox();
            deliver(&mut host, JOINER, out);
        }
        assert!(
            host.sums.len() >= 5,
            "checksums were taken: {:?}",
            host.sums.keys()
        );
        assert_eq!(host.status, PLAYING);
    }

    #[test]
    fn a_ping_comes_back_as_a_round_trip() {
        let (mut host, mut join) = started();
        for _ in 0..PING_EVERY {
            host.maybe_ping(1_000_000);
        }
        let out = host.take_outbox();
        assert!(out.iter().any(|(_, t, f)| *f && t.contains("ping")));
        deliver(&mut join, HOST, out);
        let out = join.take_outbox();
        for (_, t, _) in out {
            host.message(JOINER, &t, 1_048_000);
        }
        assert_eq!(host.stats.rtt_us, 48_000);
    }

    #[test]
    fn a_peer_that_leaves_is_reported_and_a_third_is_told_the_room_is_full() {
        let (mut host, _) = pair("b", "b");
        host.connected(3);
        let out = host.take_outbox();
        assert!(out.iter().any(|(to, t, _)| *to == 3 && t.contains(FULL)));
        host.left(JOINER);
        assert_eq!(host.status, LEFT_ROOM);
    }

    #[test]
    fn the_joiner_learns_the_start_and_the_host_learns_the_load() {
        let (mut host, mut join) = pair("b", "b");
        let load = Loadout {
            character: "breaker".into(),
            ..Loadout::default()
        };
        join.send_load(&load);
        let out = join.take_outbox();
        deliver(&mut host, JOINER, out);
        assert_eq!(host.take_events(), vec![Event::PeerLoad(load.clone())]);
        host.start(42, vec![Loadout::default(), load.clone()]);
        let out = host.take_outbox();
        deliver(&mut join, HOST, out);
        assert_eq!(
            join.take_events(),
            vec![Event::Start {
                seed: 42,
                loads: vec![Loadout::default(), load]
            }]
        );
        assert!(join.running && host.running);
    }
}
