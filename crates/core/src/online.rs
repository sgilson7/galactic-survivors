//! Online co-op, core side: what crosses the wire and the bookkeeping each
//! browser keeps. From the online kit (`kits/online/README.md`), with the
//! kit's toy world replaced by the run.
//!
//! The transport in `web/net/rtc.js` connects two browsers and carries
//! text. This module answers each event by leaving text in an outbox. Both
//! browsers run the same `World` from the same seed and both loadouts; a
//! local input is used `DELAY` ticks after it is made, and no browser runs a
//! tick until it holds both seats' inputs for it. Every `SUM_EVERY` ticks
//! each sends a checksum of its world, and a mismatch is reported.
//!
//! Every status is a key for a string in `data/strings.json`.

use crate::world::Loadout;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Bumped whenever a message changes shape or a rule changes how a tick
/// runs. `web/net/config.js` carries the same number (online_fence.rs).
pub const PROTOCOL: u32 = 1;

/// Ticks between a local input and the tick that uses it: 100 ms at
/// 60 ticks a second, time for the input to reach the other browser.
pub const DELAY: u32 = 6;

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
    Input {
        tick: u32,
        keys: u64,
    },
    Sum {
        tick: u32,
        sum: u64,
    },
    Bye {
        why: String,
    },
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

#[derive(Debug)]
pub struct Session {
    pub seat: usize,
    pub status: &'static str,
    build: String,
    pub peer: Option<Peer>,
    greeting: Option<Peer>,
    inputs: [BTreeMap<u32, u64>; SEATS],
    pub tick: u32,
    pub running: bool,
    sums: BTreeMap<u32, u64>,
    outbox: VecDeque<(Peer, String)>,
    events: VecDeque<Event>,
    /// Checksums that arrived before this browser ran their tick.
    early_sums: BTreeMap<u32, u64>,
}

impl Session {
    pub fn new(seat: usize, build: &str) -> Session {
        Session {
            seat,
            status: WAITING,
            build: build.to_string(),
            peer: None,
            greeting: None,
            inputs: Default::default(),
            tick: 0,
            running: false,
            sums: BTreeMap::new(),
            outbox: VecDeque::new(),
            events: VecDeque::new(),
            early_sums: BTreeMap::new(),
        }
    }

    fn send(&mut self, to: Peer, m: &Msg) {
        self.outbox.push_back((to, encode(m)));
    }

    pub fn take_outbox(&mut self) -> Vec<(Peer, String)> {
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
        self.tick = 0;
        self.running = true;
        self.sums.clear();
        self.early_sums.clear();
        for seat_inputs in self.inputs.iter_mut() {
            seat_inputs.clear();
            for t in 0..DELAY {
                seat_inputs.insert(t, 0);
            }
        }
        self.status = PLAYING;
    }

    pub fn message(&mut self, from: Peer, text: &str) {
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
            // inputs still in flight when a run ends are not an error
            Msg::Input { .. } | Msg::Sum { .. } if !self.running => {}
            Msg::Input { tick, keys } => {
                let other = 1 - self.seat;
                let window = self.tick..=self.tick + DELAY * 4;
                if !window.contains(&tick) || self.inputs[other].contains_key(&tick) {
                    self.status = GARBLED;
                    return;
                }
                self.inputs[other].insert(tick, keys);
            }
            Msg::Sum { tick, sum } => match self.sums.remove(&tick) {
                Some(mine) if mine != sum => self.status = DESYNC,
                Some(_) => {}
                None => {
                    self.early_sums.insert(tick, sum);
                }
            },
        }
    }

    /// This browser's input, made now, for every tick up to `DELAY` ahead
    /// that has none yet. Called once a display frame; a slow frame that
    /// runs several ticks still leaves no tick without an input.
    pub fn local(&mut self, keys: u64) {
        let Some(peer) = self.peer else { return };
        if !self.running {
            return;
        }
        for tick in self.tick..=self.tick + DELAY {
            if self.inputs[self.seat].contains_key(&tick) {
                continue;
            }
            self.inputs[self.seat].insert(tick, keys);
            self.send(peer, &Msg::Input { tick, keys });
        }
    }

    /// Both seats' inputs for the next tick, if both are here. The caller
    /// runs the tick and then calls `ran` with the world's checksum. A
    /// browser without the other's input waits; it never guesses.
    pub fn ready(&self) -> Option<[u64; SEATS]> {
        if !self.running {
            return None;
        }
        let t = self.tick;
        let a = *self.inputs[0].get(&t)?;
        let b = *self.inputs[1].get(&t)?;
        Some([a, b])
    }

    pub fn ran(&mut self, sum: u64) {
        let t = self.tick;
        for seat_inputs in self.inputs.iter_mut() {
            seat_inputs.remove(&t);
        }
        self.tick += 1;
        if self.tick.is_multiple_of(SUM_EVERY) {
            if let Some(peer) = self.peer {
                if let Some(theirs) = self.early_sums.remove(&self.tick) {
                    if theirs != sum {
                        self.status = DESYNC;
                    }
                } else {
                    self.sums.insert(self.tick, sum);
                }
                self.send(peer, &Msg::Sum { tick: self.tick, sum });
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

    /// A toy world for the session tests: two counters moved by the keys.
    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct Toy {
        pos: [i64; SEATS],
        met: u32,
    }

    impl Toy {
        fn step(&mut self, keys: [u64; SEATS]) {
            for (s, k) in keys.iter().enumerate() {
                self.pos[s] += *k as i64 % 3 - 1;
            }
            if self.pos[0] == self.pos[1] {
                self.met += 1;
            }
        }
        fn sum(&self) -> u64 {
            (self.pos[0] as u64).wrapping_mul(31) ^ (self.pos[1] as u64) ^ ((self.met as u64) << 40)
        }
    }

    fn script(seat: usize, tick: u32) -> u64 {
        ((tick * (seat as u32 + 3)) % 5) as u64
    }

    fn pair(build_host: &str, build_join: &str) -> (Session, Session) {
        let mut host = Session::new(0, build_host);
        let mut join = Session::new(1, build_join);
        host.connected(JOINER);
        join.connected(HOST);
        for (_, text) in host.take_outbox() {
            join.message(HOST, &text);
        }
        for (_, text) in join.take_outbox() {
            host.message(JOINER, &text);
        }
        (host, join)
    }

    fn started() -> (Session, Session) {
        let (mut host, mut join) = pair("b", "b");
        host.start(9, vec![Loadout::default(), Loadout::default()]);
        for (_, text) in host.take_outbox() {
            join.message(HOST, &text);
        }
        (host, join)
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
            Msg::Input {
                tick: 7,
                keys: 0xff_ffff_ffff,
            },
            Msg::Sum { tick: 30, sum: 5 },
            Msg::Bye { why: FULL.into() },
        ];
        for m in all {
            assert_eq!(decode(&encode(&m)), Some(m.clone()), "{m:?}");
        }
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
        let before = host.tick;
        for junk in [
            "",
            "{",
            "{\"t\":\"nope\"}",
            "{\"t\":\"input\",\"tick\":99999,\"keys\":1}",
        ] {
            host.status = PLAYING;
            host.message(JOINER, junk);
            assert_eq!(host.status, GARBLED, "{junk}");
            assert_eq!(host.tick, before);
        }
        // a stranger is ignored outright
        host.status = PLAYING;
        host.message(77, "{");
        assert_eq!(host.status, PLAYING);
    }

    #[test]
    fn a_copy_waits_for_the_other_seat_and_does_not_guess() {
        let (mut host, _) = started();
        for _ in 0..DELAY {
            assert!(host.ready().is_some());
            host.ran(0);
        }
        host.local(1);
        assert!(host.ready().is_none(), "ran a tick without the joiner's input");
    }

    #[test]
    fn two_lagged_copies_run_the_same_inputs_and_reach_the_same_world() {
        for lag in [0usize, 1, 3, 7] {
            let (mut host, mut join) = started();
            let (mut wh, mut wj) = (Toy::default(), Toy::default());
            let (mut used_h, mut used_j) = (Vec::new(), Vec::new());
            let mut wire_hj: VecDeque<Vec<String>> = VecDeque::new();
            let mut wire_jh: VecDeque<Vec<String>> = VecDeque::new();
            for frame in 0..600u32 {
                host.local(script(0, frame));
                join.local(script(1, frame));
                wire_hj.push_back(host.take_outbox().into_iter().map(|(_, t)| t).collect());
                wire_jh.push_back(join.take_outbox().into_iter().map(|(_, t)| t).collect());
                if wire_hj.len() > lag {
                    for t in wire_hj.pop_front().unwrap() {
                        join.message(HOST, &t);
                    }
                    for t in wire_jh.pop_front().unwrap() {
                        host.message(JOINER, &t);
                    }
                }
                while let Some(k) = host.ready() {
                    used_h.push(k);
                    wh.step(k);
                    host.ran(wh.sum());
                }
                while let Some(k) = join.ready() {
                    used_j.push(k);
                    wj.step(k);
                    join.ran(wj.sum());
                }
            }
            let n = used_h.len().min(used_j.len());
            assert!(n > 200, "lag {lag}: only {n} ticks ran");
            assert_eq!(
                used_h[..n],
                used_j[..n],
                "lag {lag}: the copies used different inputs"
            );
            assert_eq!(host.status, PLAYING, "lag {lag}");
            assert_eq!(join.status, PLAYING, "lag {lag}");
            if used_h.len() == used_j.len() {
                assert_eq!(wh, wj, "lag {lag}");
            }
        }
    }

    #[test]
    fn a_peer_that_leaves_is_reported_and_a_third_is_told_the_room_is_full() {
        let (mut host, _) = pair("b", "b");
        host.connected(3);
        let out = host.take_outbox();
        assert!(out.iter().any(|(to, t)| *to == 3 && t.contains(FULL)));
        host.left(JOINER);
        assert_eq!(host.status, LEFT_ROOM);
    }

    #[test]
    fn worlds_that_disagree_are_reported_as_a_desync() {
        let (mut host, mut join) = started();
        for _ in 0..SUM_EVERY {
            host.local(0);
            join.local(0);
            for (_, t) in host.take_outbox() {
                join.message(HOST, &t);
            }
            for (_, t) in join.take_outbox() {
                host.message(JOINER, &t);
            }
            while host.ready().is_some() {
                host.ran(1);
            }
            while join.ready().is_some() {
                join.ran(2);
            }
        }
        for (_, t) in join.take_outbox() {
            host.message(JOINER, &t);
        }
        assert_eq!(host.status, DESYNC);
    }

    #[test]
    fn the_joiner_learns_the_start_and_the_host_learns_the_load() {
        let (mut host, mut join) = pair("b", "b");
        let load = Loadout {
            character: "breaker".into(),
            ..Loadout::default()
        };
        join.send_load(&load);
        for (_, t) in join.take_outbox() {
            host.message(JOINER, &t);
        }
        assert_eq!(host.take_events(), vec![Event::PeerLoad(load.clone())]);
        host.start(42, vec![Loadout::default(), load.clone()]);
        for (_, t) in host.take_outbox() {
            join.message(HOST, &t);
        }
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
