"use strict";
const canvas = document.querySelector("#game"),
  ctx = canvas.getContext("2d"),
  stage = document.querySelector("#stage"),
  entry = document.querySelector("#entry"),
  error = document.querySelector("#error");
const token = document.querySelector("meta[name=totengine-token]").content;
let frame = null,
  scale = 1,
  busy = false,
  editPending = false,
  pendingKey = null,
  pendingFocus = null,
  pendingDoubleClick = null,
  inFlight = null,
  lastClick = null,
  assets = "",
  sprites = new Map(),
  counter = 0,
  paintedRevision = -1;
const prefix = crypto.getRandomValues(new Uint32Array(1))[0] >>> 0;
function id() {
  return prefix * 1048576 + ++counter;
} // below Number.MAX_SAFE_INTEGER
function resize() {
  scale = Math.min(innerWidth / 1024, innerHeight / 768);
  stage.style.transform = `translate(${(innerWidth - 1024 * scale) / 2}px,${(innerHeight - 768 * scale) / 2}px) scale(${scale})`;
}
addEventListener("resize", resize);
resize();
function showError(s) {
  error.textContent = s;
  error.style.display = s ? "block" : "none";
}
async function sprite(key) {
  if (sprites.has(key)) return sprites.get(key);
  const promise = (async () => {
    let r = await fetch(`/sprite/${assets}/${key}`);
    if (!r.ok) throw Error("Artwork unavailable: " + key);
    const buffer = await r.arrayBuffer(),
      v = new DataView(buffer),
      w = v.getUint32(0, true),
      h = v.getUint32(4, true);
    if (w * h * 4 + 8 !== buffer.byteLength) throw Error("Invalid sprite");
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    c.getContext("2d").putImageData(
      new ImageData(new Uint8ClampedArray(buffer, 8), w, h),
      0,
      0,
    );
    return c;
  })();
  sprites.set(key, promise);
  return promise;
}
let paintedFrame = null, paintedImages = new Map();
function drawCanvas() {
  if (!paintedFrame) return;
  ctx.imageSmoothingEnabled = false;
  for (const d of paintedFrame.draws) {
    switch (d.kind) {
      case "rect":
        ctx.fillStyle = d.color;
        ctx.fillRect(d.x, d.y, d.w, d.h);
        break;
      case "text":
        ctx.fillStyle = d.color;
        ctx.font = `${d.size}px Georgia,serif`;
        ctx.textBaseline = "top";
        ctx.fillText(d.text, d.x, d.y);
        break;
      case "tile":
        ctx.save();ctx.translate(d.x,d.y);ctx.fillStyle=ctx.createPattern(paintedImages.get(d.sprite),"repeat");ctx.fillRect(0,0,d.w,d.h);ctx.restore();break;
      case "unit_sprite":
        if (d.blink && Math.floor(performance.now()/500)%2) break;
        ctx.drawImage(paintedImages.get(d.sprite), d.x, d.y, d.w, d.h);
        break;
      case "sprite":
        ctx.drawImage(paintedImages.get(d.sprite), d.x, d.y, d.w, d.h);
        break;
      case "cursor":
        if (d.blink && Math.floor(performance.now()/500)%2) break;
        ctx.strokeStyle = d.color; ctx.lineWidth = 1;
        ctx.beginPath();ctx.moveTo(d.x-d.w/2,d.y);ctx.lineTo(d.x,d.y-d.h/2);
        ctx.lineTo(d.x+d.w/2,d.y);ctx.lineTo(d.x,d.y+d.h/2);ctx.closePath();ctx.stroke();
        break;
      case "line":
        ctx.strokeStyle = d.color;
        ctx.lineWidth = d.width;
        ctx.beginPath();
        ctx.moveTo(d.x, d.y);
        ctx.lineTo(d.x2, d.y2);
        ctx.stroke();
        break;
    }
  }
}
// Animation only redraws the canvas. No input, network, DOM editing or game tick.
setInterval(() => {
  if (paintedFrame?.draws.some(d => (d.kind === "unit_sprite" || d.kind === "cursor") && d.blink)) drawCanvas();
}, 100);
let paintSerial = 0;
async function paint(next) {
  if (frame && next.revision < frame.revision) return;
  frame = next;
  if (assets !== next.assets) {
    assets = next.assets;
    sprites.clear();
  }
  const serial = ++paintSerial;
  const keys = [
    ...new Set(
      next.draws.filter((d) => d.kind === "sprite" || d.kind === "unit_sprite" || d.kind === "tile").map((d) => d.sprite),
    ),
  ];
  const images = await Promise.all(keys.map(sprite));
  const ready = new Map(keys.map((k, i) => [k, images[i]]));
  if (serial !== paintSerial) return;
  paintedFrame = next;
  paintedImages = ready;
  drawCanvas();
  const meta = next.entry;
  if (meta) {
    if (entry.dataset.field !== meta.id) {
      entry.value = meta.value;
      entry.dataset.field = meta.id;
      editPending = false;
    } else if (!editPending) entry.value = meta.value;
    Object.assign(entry.style, {
      left: meta.x + "px",
      top: meta.y + "px",
      width: meta.w + "px",
      height: meta.h + "px",
    });
    entry.hidden = false;
    if (document.activeElement !== entry) entry.focus({ preventScroll: true });
  } else {
    entry.hidden = true;
    entry.dataset.field = "";
    if (document.activeElement === entry) canvas.focus();
  }
  paintedRevision = next.revision;

}
async function refresh() {
  const r = await fetch("/state");
  if (!r.ok) throw Error("Host unavailable");
  await paint(await r.json());
}
async function send(input, requestId = id()) {
  if (!frame) return;
  if (busy) {
    if (input.type === "focus") pendingFocus = input.focused;
    else if (input.type === "double_click" && inFlight?.id === input.first_click && inFlight.input.type === "click")
      pendingDoubleClick = input;
    else if (input.type === "edit") editPending = true;
    else if (input.type === "key" && !entry.hidden)
      pendingKey = { input, field: entry.dataset.field };
    else showError("Input is pending; wait for the refreshed selection");
    return;
  }
  busy = true;
  const request = { revision: frame.revision, id: requestId, input };
  inFlight = request;
  try {
    let r; // A network retry keeps the identical ID, revision and payload.
    for (let n = 0; n < 2; n++) {
      try {
        r = await fetch("/input", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Totengine-Token": token,
          },
          body: JSON.stringify(request),
        });
        break;
      } catch (e) {
        if (n === 1) throw e;
      }
    }
    const data = await r.json();
    if (!r.ok) {
      showError(data.error || "Input rejected");
      await refresh();
    } else {
      showError("");
      await paint(data);
      if (data.phase === "Opponents" && data.advancing && ["click","key","tick"].includes(input.type)) setTimeout(()=>send({type:"tick"}),20);
    }
  } catch (e) {
    showError(e.message + " — reconnect to retrieve the pending decision");
  } finally {
    busy = false;
    inFlight = null;
    if (pendingDoubleClick && frame.city_click !== pendingDoubleClick.first_click)
      pendingDoubleClick = null;
    if (pendingFocus !== null) {
      const focused = pendingFocus;
      pendingFocus = null;
      // Begin immediately: a newer focus event must not be overtaken by an
      // older transition waiting in a timer callback.
      send({ type: "focus", focused });
    } else if (pendingDoubleClick) {
      const pending = pendingDoubleClick;
      pendingDoubleClick = null;
      if (frame.city_click === pending.first_click) send(pending);
    } else if (editPending && !entry.hidden) {
      editPending = false;
      const field = entry.dataset.field,
        value = entry.value;
      setTimeout(() => send({ type: "edit", field, value }), 0);
    } else if (pendingKey) {
      const pending = pendingKey;
      pendingKey = null;
      if (!entry.hidden && entry.dataset.field === pending.field)
        setTimeout(() => send(pending.input), 0);
    }
  }
}
canvas.addEventListener("click", (e) => {
  const r = canvas.getBoundingClientRect();
  const x = (e.clientX - r.left) / scale, y = (e.clientY - r.top) / scale;
  if (e.detail > 1 && lastClick && (frame.city_click === lastClick || inFlight?.id === lastClick)) {
    send({type:"double_click", x, y, first_click:lastClick});
  } else {
    lastClick = id();
    send({type:"click", x, y}, lastClick);
  }
});
entry.addEventListener("input", () => {
  editPending = true;
  if (!busy) {
    editPending = false;
    send({ type: "edit", field: entry.dataset.field, value: entry.value });
  }
});
addEventListener("keydown", (e) => {
  if (e.repeat || e.altKey) return;
  const editing = document.activeElement === entry;
  if (editing && !["Enter", "Escape", "Tab"].includes(e.key)) return;
  const modified = e.ctrlKey || e.metaKey;
  let key = e.key === "Tab" && e.shiftKey ? "ShiftTab" :
    e.key.length === 1 ? e.key.toLowerCase() : e.key;
  // Physical keypad position works with Num Lock either on or off. Do not
  // translate these while a text field owns input or a modifier is held.
  if (!editing && !modified && /^Numpad[1-9]$/.test(e.code)) key = e.code.slice(6);
  if (modified) {
    if (editing || !["s", "l", "n"].includes(key) || e.shiftKey) return;
    key = "Ctrl+" + key;
  } else if (e.shiftKey && key === "z") key = "Shift+z";
  else if (e.shiftKey && /^[a-z]$/.test(key)) return;
  if (["Enter", "Escape", "Tab", "ShiftTab", "ArrowLeft", "ArrowRight",
       "ArrowUp", "ArrowDown", "PageUp", "PageDown", " ", "+", "-", "=",
       "Ctrl+s", "Ctrl+l", "Ctrl+n", "Shift+z"].includes(key) ||
      (!editing && /^[abvcwzxfremosl123456789]$/.test(key))) {
    e.preventDefault();
    send({ type: "key", key });
  }
});
let focusEpoch = 0;
addEventListener("blur", () => {
  ++focusEpoch;
  send({ type: "focus", focused: false });
});
addEventListener("focus", async () => {
  const epoch = ++focusEpoch;
  try {
    await refresh();
    // A later blur (or focus) owns the state even if this refresh finishes last.
    if (epoch === focusEpoch) send({ type: "focus", focused: true });
  } catch (e) {
    if (epoch === focusEpoch) showError(e.message);
  }
});
setInterval(() => {
  if (frame?.phase !== "Closed" && !busy && !editPending)
    refresh().catch((e) => showError(e.message));
}, 1500);
refresh().catch((e) => showError(e.message));
