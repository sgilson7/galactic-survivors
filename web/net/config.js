// Online settings: the only file in the project that names an address on
// another origin (kits/online). Read only by web/net/rtc.js, and rtc.js runs
// only after a player presses Host or Join; loading and idling the page
// never reads it.
//
// crates/core/tests/online_fence.rs holds this file to three things:
//   - it is the only file outside web/net/vendor/ that names a wss:// address;
//   - every page's CSP connect-src is 'self' plus exactly `relays` below;
//   - `protocol` equals PROTOCOL in crates/core/src/online.rs.
//
// The page scan allows numbers here: each is a transport timing, an id or a
// byte value, never a rule of the game.

export const CONFIG = {
  // Keeps this game's rooms apart from every other app on the same relays.
  appId: "galactic-survivors-p2p",

  // The core's PROTOCOL. The room name is appId, this tag and the code.
  protocol: 2,

  // The vendored Trystero bundle, pinned in vendor/SHA256SUMS and checked by
  // scripts/build.sh. Imported only when a player uses a room code.
  bundle: "net/vendor/trystero-nostr-0.25.4.js",

  // Encrypts session descriptions while they cross relays this project
  // does not run. It ships in this file, so it is not a secret.
  password: "galactic-survivors",

  // The Nostr relays that introduce players by room code: the list Raiders
  // of the Lost Mart found answering in October 2026. Check each answers a
  // WebSocket upgrade before a release.
  relays: [
    "wss://nos.lol",
    "wss://nostr.mom",
    "wss://relay.damus.io",
    "wss://relay.primal.net",
    "wss://nostr-01.yakihonne.com",
    "wss://nostr.sathoarder.com",
    "wss://relay.mostro.network",
  ],

  // STUN tells a browser its public address. Without TURN, two players
  // both behind strict NATs cannot connect (online.no_path says so).
  iceServers: [
    { urls: "stun:stun.l.google.com:19302" },
    { urls: "stun:global.stun.twilio.com:3478" },
  ],

  // The negotiated data channel's stream id. Not 0 or 1: Trystero opens its
  // own in-band channel from 0 upwards.
  channelId: 40,

  // The fast channel's stream id: unordered and never resent, for inputs
  // and pings only. A lost input is carried again by the next message, so
  // nothing waits behind it (T13).
  fastChannelId: 41,

  // The first byte of every frame: who sent it, an acknowledgement, or data.
  frame: { host: 0x48, joiner: 0x4a, ack: 0x41, data: 0x44 },

  // How often a link repeats its role until the other end acknowledges it.
  helloRepeatMs: 250,

  // How long to gather ICE candidates before a pasted code is shown anyway.
  gatherTimeoutMs: 6000,

  // How long a room code waits for any relay before reporting online.no_relay.
  relayTimeoutMs: 15000,
};

// `?ice=none` drops the STUN servers, so tabs on one machine connect on host
// candidates alone and nothing leaves the origin. tests/browser/online.py
// uses it; a player never needs it.
export function iceServers() {
  const ice = new URLSearchParams(location.search).get("ice");
  return ice === "none" ? [] : CONFIG.iceServers;
}
