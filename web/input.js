// Keyboard, mouse and touch, turned into what the core reads each frame:
// action bits, a stick and a pointer. After the realtime kit's input.js and
// the mobile kit (M1): devices become actions, and only actions become
// input. This file decides nothing about what an action does.
//
// data/controls.json names an action for each key code; the core's assets
// give each action its bit. A press released before the frame reads it
// still reaches the core once (`tapped`).
//
// Mouse: the pointer's position on the canvas, for aimed abilities.
// Touch: a drag anywhere on the canvas is a stick, from where the finger
// went down to where it is now.

export function makeInput(canvas, controls, bits, knob) {
  const state = { held: 0, tapped: 0, px: 0, py: 0, pointer: false, sx: 0, sy: 0, touch: null };
  const bitOf = (code) => bits[controls[code]] || 0;

  function typing(e) {
    const t = e.target;
    return t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA");
  }

  window.addEventListener("keydown", (e) => {
    if (typing(e)) return;
    const bit = bitOf(e.code);
    if (!bit) return;
    if (!e.repeat) state.tapped |= bit;
    state.held |= bit;
    e.preventDefault();
  });
  window.addEventListener("keyup", (e) => {
    state.held &= ~bitOf(e.code);
  });
  window.addEventListener("blur", () => {
    state.held = 0;
    state.sx = 0;
    state.sy = 0;
    state.touch = null;
    knob.hidden = true;
  });

  canvas.addEventListener("pointermove", (e) => {
    if (e.pointerType === "mouse") {
      state.px = e.offsetX;
      state.py = e.offsetY;
      state.pointer = true;
      return;
    }
    if (!state.touch || state.touch.id !== e.pointerId) return;
    state.sx = e.offsetX - state.touch.x;
    state.sy = e.offsetY - state.touch.y;
  });
  canvas.addEventListener("pointerleave", (e) => {
    if (e.pointerType === "mouse") state.pointer = false;
  });
  canvas.addEventListener("pointerdown", (e) => {
    if (e.pointerType === "mouse") return;
    try {
      canvas.setPointerCapture(e.pointerId);
    } catch (err) {
      // a refused capture still steers while the finger stays on the canvas
    }
    state.touch = { id: e.pointerId, x: e.offsetX, y: e.offsetY };
    state.sx = 0;
    state.sy = 0;
    knob.style.left = `${e.offsetX}px`;
    knob.style.top = `${e.offsetY}px`;
    knob.hidden = false;
    e.preventDefault();
  });
  const lift = (e) => {
    if (!state.touch || state.touch.id !== e.pointerId) return;
    state.touch = null;
    state.sx = 0;
    state.sy = 0;
    knob.hidden = true;
  };
  canvas.addEventListener("pointerup", lift);
  canvas.addEventListener("pointercancel", lift);
  canvas.addEventListener("lostpointercapture", lift);
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());

  return {
    // Once per display frame.
    read() {
      const out = {
        bits: state.held | state.tapped,
        sx: state.sx,
        sy: state.sy,
        px: state.px,
        py: state.py,
        pointer: state.pointer,
      };
      state.tapped = 0;
      return out;
    },
  };
}
