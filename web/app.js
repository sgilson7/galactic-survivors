// The page draws what the core returns and decides nothing.
//
// Every display frame it hands the core the time and the input, replays
// the core's draw list on the canvas, and rebuilds the menus when the
// core's screen description changes. Buttons send their action back
// through `act`. Actions named `page.*` are input and output the core
// cannot do itself (save a file, read a file, open the co-op transport);
// the page does them and reports back through `act`.
import init, { Galaxy } from "./pkg/tool_shim.js";
import { createRenderer } from "./render2d.js";
import { makeInput } from "./input.js";

const el = (id) => document.getElementById(id);
const make = (tag, text, cls) => {
  const node = document.createElement(tag);
  if (text !== undefined && text !== "") node.textContent = text;
  if (cls) node.className = cls;
  return node;
};
const word = () => crypto.getRandomValues(new Uint32Array(1))[0];

let game = null;
let renderer = null;
let input = null;
let online = null;
let lastUi = "";
let size = { width: 0, height: 0 };
let build = "";

function say(key) {
  return game ? game.say(key) : key;
}

function message(text) {
  el("message").textContent = text || "";
}

function act(name, value) {
  if (name.startsWith("page.")) return page(name);
  const result = JSON.parse(game.act(name, value || ""));
  if (result.error) message(result.error);
  else message("");
  render(true);
}

// ---- input and output the core asks for --------------------------------------

function download() {
  const blob = new Blob([game.save_text()], { type: "application/json" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = game.save_name();
  document.body.append(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(a.href);
  act("save.downloaded");
}

function load() {
  const picker = el("file");
  picker.value = "";
  picker.onchange = async () => {
    const file = picker.files && picker.files[0];
    if (!file) return message(say("page.no_file"));
    act("save.loaded", await file.text());
  };
  picker.click();
}

async function transport() {
  if (!online) {
    const { makeOnline } = await import("./online.js");
    online = makeOnline(game, build);
  }
  return online;
}

// A small form under the menus: a prompt, a text box, and buttons.
function ask(promptKey, value, buttons) {
  const box = el("ask");
  box.replaceChildren();
  box.append(make("p", say(promptKey)));
  const area = make("textarea");
  area.id = "ask-text";
  area.value = value || "";
  area.rows = 1;
  box.append(area);
  const row = make("div", undefined, "actions");
  for (const [label, fn] of buttons) {
    const b = make("button", label);
    b.addEventListener("click", () => fn(area.value, area));
    row.append(b);
  }
  box.append(row);
  box.hidden = false;
  area.focus();
}

function closeAsk() {
  el("ask").hidden = true;
}

function copy(text) {
  if (navigator.clipboard) navigator.clipboard.writeText(text).then(() => message(say("coop.copied")), () => {});
}

async function page(name) {
  if (name === "page.download") return download();
  if (name === "page.load") return load();
  const net = await transport();
  if (name === "page.host") {
    net.host();
  } else if (name === "page.join") {
    ask("coop.code_prompt", "", [
      [say("coop.join"), (code) => {
        if (net.join(code)) closeAsk();
        else message(say("online.bad_code"));
      }],
    ]);
  } else if (name === "page.invite") {
    message(say("page.loading"));
    const invitation = await net.invite();
    message("");
    ask("coop.invite_prompt", invitation, [
      [say("coop.copied"), (_, area) => copy(invitation) || area.select()],
      [say("coop.start"), async (_, area) => {
        try {
          await net.accept(area.value);
          closeAsk();
        } catch (e) {
          message(say("online.signal_failed"));
        }
      }],
    ]);
    el("ask-text").addEventListener("focus", (e) => e.target.select());
  } else if (name === "page.reply") {
    ask("coop.reply_prompt", "", [
      [say("coop.reply"), async (code) => {
        try {
          const reply = await net.reply(code);
          ask("coop.invite_prompt", reply, [[say("coop.copied"), () => copy(reply)]]);
        } catch (e) {
          message(say("online.signal_failed"));
        }
      }],
    ]);
  }
  render(true);
}

// ---- the screen ------------------------------------------------------------

function button(a) {
  const b = make("button", undefined, a.primary ? "primary" : "");
  b.append(make("span", a.label));
  if (a.hint) b.append(make("kbd", a.hint));
  b.disabled = !a.enabled;
  b.dataset.action = a.name;
  b.dataset.value = a.value;
  b.addEventListener("click", () => act(a.name, a.value));
  return b;
}

function item(it) {
  const box = make("article", undefined, "item");
  box.id = `item-${it.id}`;
  if (it.tone) box.dataset.tone = it.tone;
  if (it.selected) box.dataset.selected = "true";
  if (it.at) {
    box.style.gridColumn = String(it.at[0] + 1);
    box.style.gridRow = String(it.at[1] + 1);
  }
  if (it.click) {
    // a tile: the whole item is one button
    box.tabIndex = 0;
    box.setAttribute("role", "button");
    box.setAttribute("aria-label", it.click.label);
    box.addEventListener("click", () => act(it.click.name, it.click.value));
    box.addEventListener("keydown", (e) => {
      if (e.code === "Enter" || e.code === "Space") {
        e.preventDefault();
        act(it.click.name, it.click.value);
      }
    });
  }
  if (it.badge) box.append(make("span", it.badge, "badge"));
  if (it.icon) {
    const img = make("img");
    img.src = `art/${it.icon}.png`;
    img.alt = "";
    box.append(img);
  }
  const body = make("div", undefined, "body");
  const head = make("div", undefined, "head");
  head.append(make("h3", it.title));
  if (it.tag) head.append(make("span", it.tag, "tag"));
  body.append(head);
  if (it.text) body.append(make("p", it.text));
  if (it.progress !== undefined) {
    const bar = make("div", undefined, "bar");
    const fill = make("i");
    fill.style.width = `${it.progress}%`;
    bar.append(fill);
    body.append(bar);
  }
  if (it.actions && it.actions.length) {
    const row = make("div", undefined, "actions");
    it.actions.forEach((a) => row.append(button(a)));
    body.append(row);
  }
  box.append(body);
  return box;
}

function panel(p) {
  const box = make("section", undefined, "panel");
  box.id = `panel-${p.id}`;
  box.dataset.kind = p.kind || "";
  if (p.heading) box.append(make("h2", p.heading));
  if (p.text) box.append(make("p", p.text, "lead"));
  if (p.items && p.items.length) {
    const list = make("div", undefined, "items");
    p.items.forEach((it) => list.append(item(it)));
    box.append(list);
  }
  if (p.actions && p.actions.length) {
    const row = make("div", undefined, "actions");
    p.actions.forEach((a) => row.append(button(a)));
    box.append(row);
  }
  return box;
}

function render(force) {
  const text = game.ui();
  if (!force && text === lastUi) return;
  lastUi = text;
  const result = JSON.parse(text);
  if (result.error) return message(result.error);
  const ui = result.ok;
  document.body.dataset.screen = ui.screen;
  document.body.dataset.motion = ui.reduced_motion ? "reduced" : "full";
  el("title").textContent = ui.title;
  el("subtitle").textContent = ui.subtitle;
  el("panels").replaceChildren(...ui.panels.filter((p) => ui.screen !== "run").map(panel));
  el("runbar").replaceChildren(...ui.panels.filter((p) => ui.screen === "run").map(panel));
  el("overlay").replaceChildren(...(ui.overlay ? [panel(ui.overlay)] : []));
  el("overlay").hidden = !ui.overlay;
  el("build").replaceChildren(...(ui.build || []).map(item));
  if (ui.notice) message(ui.notice);
}

// Enter takes the first primary action on the overlay or the menu.
window.addEventListener("keydown", (e) => {
  if (e.code !== "Enter" || e.target.tagName === "TEXTAREA" || e.target.tagName === "BUTTON") return;
  const scope = el("overlay").hidden ? el("panels") : el("overlay");
  const b = scope.querySelector("button.primary:not([disabled])");
  if (b) b.click();
});

function frame(now) {
  const canvas = el("view");
  if (canvas.clientWidth !== size.width || canvas.clientHeight !== size.height) {
    size = renderer.resize();
    game.resize(size.width, size.height);
  }
  const i = input.read();
  game.advance(now, i.bits, i.sx, i.sy, i.px, i.py, i.pointer);
  if (online) {
    online.pump();
    document.body.dataset.net = game.net_status();
  }
  document.body.dataset.tick = game.tick();
  if (document.body.dataset.screen === "run") renderer.draw(game.draw_list());
  render(false);
  requestAnimationFrame(frame);
}

async function main() {
  await init();
  const [content, strings, controls, stamp] = await Promise.all([
    fetch("data/content.json").then((r) => r.text()),
    fetch("data/strings.json").then((r) => r.text()),
    fetch("data/controls.json").then((r) => r.json()),
    fetch("build.txt").then((r) => (r.ok ? r.text() : "dev"), () => "dev"),
  ]);
  build = stamp.trim().split(" ")[0];
  game = new Galaxy(content, strings, word(), word(), build);
  if (game.error()) {
    message(game.error());
    return;
  }
  const assets = JSON.parse(game.assets());
  const canvas = el("view");
  renderer = createRenderer(canvas, assets.draw);
  input = makeInput(canvas, controls, assets.bits, el("knob"));
  render(true);
  document.body.dataset.ready = "true";
  requestAnimationFrame(frame);
}

main();
