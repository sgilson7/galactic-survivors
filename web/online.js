// Online co-op, as the page sees it: opens the transport (web/net/rtc.js),
// hands the core every peer, departure and message, and sends what the
// core leaves in its outbox. The core decides everything; this file only
// carries. It is the one page file that imports the transport
// (online_fence.rs), and app.js loads it only when a player hosts or joins.
import * as rtc from "./net/rtc.js";
import { CONFIG } from "./net/config.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const word = () => crypto.getRandomValues(new Uint32Array(1))[0];

export function makeOnline(game, build) {
  let link = null;

  function open(isHost, mode, room) {
    close();
    link = rtc.open({
      isHost,
      mode,
      room,
      build,
      onPeer: (id) => game.net_peer(id),
      onLeft: (id) => game.net_left(id),
      onMessage: (id, bytes) => game.net_message(id, decoder.decode(bytes)),
      onError: (key) => game.net_error(key),
    });
    return link;
  }

  function close() {
    if (link) link.close();
    link = null;
  }

  return {
    // Host with a room code over the public relays in net/config.js.
    host() {
      open(true, "room", game.host(word(), word(), CONFIG.appId));
    },
    // Join by a typed room code; false when it cannot be one.
    join(code) {
      const room = game.join(code, CONFIG.appId);
      if (!room) return false;
      open(false, "room", room);
      return true;
    },
    // Host by pasted codes: resolves to the invitation to send.
    invite() {
      game.host(word(), word(), CONFIG.appId);
      return open(true, "code", "").invitation();
    },
    accept: (reply) => (link ? link.accept(reply) : Promise.reject(new Error("not hosting"))),
    // Answer a pasted invitation: resolves to the reply to send back.
    reply(invitation) {
      game.join("", CONFIG.appId);
      return open(false, "code", "").reply(invitation);
    },
    pump() {
      if (!link) return;
      for (const [id, text] of JSON.parse(game.net_outbox())) link.send(id, encoder.encode(text));
      if (!game.net_active()) close();
    },
    close,
  };
}
