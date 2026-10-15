// Replays the core's draw list onto a 2D canvas (after the canvas kit and
// Peter's renderer for Mandible). It holds no rule and no number: op codes,
// palette roles, picture names and units come from the core's assets, read
// once, and colours come from the stylesheet (role `ink` is `--ink`).

export function createRenderer(canvas, assets) {
  const ctx = canvas.getContext("2d");
  const images = assets.sprites.map((name) => {
    const img = new Image();
    img.src = `art/${name}.png`;
    return img;
  });
  const groundImage = new Image();
  groundImage.src = `art/${assets.ground}.png`;
  let pattern = null;
  let colors = [];
  let family = "";
  let width = 0;
  let height = 0;
  let list = [];
  let at = 0;
  const full = assets.tau / assets.unit;
  const angle = (turns) => (turns / assets.turn) * full;

  function readStyle() {
    const style = getComputedStyle(canvas);
    colors = assets.palette.map((role) => style.getPropertyValue(`--${role}`).trim());
    family = style.getPropertyValue("--hud-font").trim() || style.fontFamily;
  }

  // The canvas fills its box; the backing store matches the screen's
  // density so lines stay sharp. Returns the size in CSS pixels.
  function resize() {
    const density = window.devicePixelRatio || 1;
    width = canvas.clientWidth;
    height = canvas.clientHeight;
    canvas.width = width * density;
    canvas.height = height * density;
    ctx.setTransform(density, 0, 0, density, 0, 0);
    return { width, height };
  }

  const next = () => {
    const value = list[at];
    at += 1;
    return value;
  };
  const alpha = (a) => {
    ctx.globalAlpha = a / assets.unit;
  };

  const ops = [];
  ops[assets.ops.clear] = () => {
    ctx.globalAlpha = 1;
    ctx.fillStyle = colors[next()];
    ctx.fillRect(0, 0, width, height);
  };
  ops[assets.ops.rect] = () => {
    const x = next();
    const y = next();
    const w = next();
    const h = next();
    ctx.fillStyle = colors[next()];
    alpha(next());
    ctx.fillRect(x, y, w, h);
  };
  ops[assets.ops.circle] = () => {
    const x = next();
    const y = next();
    const r = next();
    const fill = colors[next()];
    const stroke = colors[next()];
    const lineWidth = next();
    alpha(next());
    ctx.beginPath();
    ctx.arc(x, y, r, 0, full);
    ctx.fillStyle = fill;
    ctx.fill();
    if (lineWidth > 0) {
      ctx.lineWidth = lineWidth;
      ctx.strokeStyle = stroke;
      ctx.stroke();
    }
  };
  ops[assets.ops.sprite] = () => {
    const img = images[next()];
    const x = next();
    const y = next();
    const size = next();
    const flip = next();
    alpha(next());
    if (!img || !img.complete || !img.naturalWidth) return;
    if (flip) {
      ctx.save();
      ctx.translate(x + size, y);
      ctx.scale(-1, 1);
      ctx.drawImage(img, 0, 0, size, size);
      ctx.restore();
    } else {
      ctx.drawImage(img, x, y, size, size);
    }
  };
  ops[assets.ops.line] = () => {
    ctx.strokeStyle = colors[next()];
    ctx.lineWidth = next();
    alpha(next());
    const count = next();
    ctx.beginPath();
    for (let k = 0; k < count; k += 1) {
      const x = next();
      ctx.lineTo(x, next());
    }
    ctx.stroke();
  };
  ops[assets.ops.text] = () => {
    const x = next();
    const y = next();
    const size = next();
    ctx.fillStyle = colors[next()];
    ctx.textAlign = assets.aligns[next()];
    const count = next();
    const codes = [];
    for (let k = 0; k < count; k += 1) codes.push(next());
    ctx.globalAlpha = 1;
    ctx.font = `600 ${size}px ${family}`;
    ctx.fillText(String.fromCodePoint(...codes), x, y);
  };
  ops[assets.ops.ground] = () => {
    const dx = next();
    const dy = next();
    const size = next();
    ctx.globalAlpha = 1;
    if (!groundImage.complete || !groundImage.naturalWidth) return;
    if (!pattern) pattern = ctx.createPattern(groundImage, "repeat");
    pattern.setTransform(new DOMMatrix().translate(dx, dy).scale(size / groundImage.naturalWidth));
    ctx.fillStyle = pattern;
    ctx.fillRect(0, 0, width, height);
  };
  ops[assets.ops.wedge] = () => {
    const x = next();
    const y = next();
    const r = next();
    const from = next();
    const to = next();
    ctx.fillStyle = colors[next()];
    alpha(next());
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.arc(x, y, r, angle(from), angle(to));
    ctx.closePath();
    ctx.fill();
  };

  function draw(drawList) {
    list = drawList;
    at = 0;
    ctx.textBaseline = "middle";
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    while (at < list.length) ops[next()]();
    ctx.globalAlpha = 1;
  }

  readStyle();
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", readStyle);
  return { draw, resize, readStyle };
}
