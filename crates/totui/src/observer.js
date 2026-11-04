"use strict";
const $ = (id) => document.getElementById(id);
const colors = [
  "#ddd",
  "#ff7777",
  "#f5db72",
  "#83aaff",
  "#7cdb9a",
  "#76d5dd",
  "#d6a0f4",
  "#f5af6c",
];
const terrain = [
  "#bda46b",
  "#9da86a",
  "#6a9d59",
  "#64835a",
  "#548264",
  "#807a68",
  "#a4a5a0",
  "#8eab9e",
  "#c6d3c8",
  "#476e57",
  "#365d80",
];
let frames = [],
  total = 0,
  index = 0,
  offline = false,
  playing = false,
  busy = false,
  target = 0,
  timer;
const canvas = $("map"),
  ctx = canvas.getContext("2d"),
  tileW = 32,
  tileH = 20;
async function json(url) {
  const r = await fetch(url);
  if (!r.ok) throw Error(`Request failed (${r.status})`);
  return r.json();
}
function pause() {
  target = index;
  playing = false;
  clearTimeout(timer);
  $("play").textContent = "Play";
}
function fail(e) {
  pause();
  $("error").textContent = e.message;
}
function draw() {
  const f = frames[index];
  if (!f) return;
  const v = f.views[Number($("fog").value)];
  canvas.width = f.width * tileW + tileW / 2;
  canvas.height = ((f.height + 1) * tileH) / 2;
  ctx.fillStyle = "#080e15";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  for (let i = 0; i < v.tiles.length; i++) {
    const t = v.tiles[i],
      row = Math.floor(i / f.width),
      x = (i % f.width) * tileW + ((row % 2) * tileW) / 2,
      y = (row * tileH) / 2;
    ctx.beginPath();
    ctx.moveTo(x + tileW / 2, y);
    ctx.lineTo(x + tileW, y + tileH / 2);
    ctx.lineTo(x + tileW / 2, y + tileH);
    ctx.lineTo(x, y + tileH / 2);
    ctx.closePath();
    ctx.fillStyle = t ? terrain[t[0]] || "#666" : "#15202c";
    ctx.fill();
    ctx.strokeStyle = "#0002";
    ctx.stroke();
    if (t && (t[1] || t[2] || t[3])) {
      ctx.strokeStyle = t[3] ? "#ddd" : t[2] ? "#d1bc85" : "#77b6d5";
      ctx.beginPath();
      ctx.moveTo(x + 8, y + 10);
      ctx.lineTo(x + 24, y + 10);
      ctx.stroke();
    }
  }
  const pos = (o) => [
    (o.x * tileW) / 2 + tileW / 2,
    (o.y * tileH) / 2 + tileH / 2,
  ];
  ctx.font = "10px system-ui";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (const u of v.units) {
    const [x, y] = pos(u);
    ctx.fillStyle = colors[u.owner] || "#fff";
    ctx.beginPath();
    ctx.arc(x, y, 6, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#101820";
    ctx.fillText(u.type, x, y);
  }
  for (const c of v.cities) {
    const [x, y] = pos(c);
    ctx.fillStyle = "#121a23";
    ctx.fillRect(x - 9, y - 8, 18, 16);
    ctx.fillStyle = colors[c.owner] || "#fff";
    ctx.fillRect(x - 7, y - 6, 14, 12);
    ctx.fillStyle = "#101820";
    ctx.fillText(c.size, x, y);
  }
  $("status").textContent =
    `Turn ${f.turn} · Year ${f.year} · ${v.units.length} units · ${v.cities.length} cities · ${index + 1}/${total}`;
  $("status").title = `Checksum ${f.checksum}`;
  $("timeline").max = Math.max(0, total - 1);
  $("timeline").value = index;
  $("back").disabled = index === 0;
  $("next").disabled = index >= total - 1;
  $("events").textContent = f.events.join("\n");
  $("orderlog").hidden = offline;
}
async function go(n) {
  target = Math.max(0, Math.min(n, total - 1));
  if (busy) return;
  busy = true;
  try {
    // Only request successive new frames: the server advances once for each.
    while (!offline && frames.length <= target)
      frames.push(await json(`/frame/${frames.length}`));
    index = target;
    $("error").textContent = "";
    draw();
    if (index >= total - 1) pause();
  } catch (e) {
    fail(e);
    draw();
  } finally {
    busy = false;
  }
}
async function tick() {
  if (!playing) return;
  await go(index + 1);
  if (playing) timer = setTimeout(tick, Number($("speed").value));
}
$("play").onclick = () => {
  if (playing) pause();
  else {
    playing = true;
    $("play").textContent = "Pause";
    tick();
  }
};
$("next").onclick = () => {
  pause();
  go(index + 1);
};
$("back").onclick = () => {
  pause();
  go(index - 1);
};
$("restart").onclick = () => {
  pause();
  go(0);
};
$("timeline").oninput = () => {
  pause();
  go(Number($("timeline").value));
};
$("fog").onchange = draw;
canvas.onmousemove = (e) => {
  const f = frames[index];
  if (!f) return;
  const rect = canvas.getBoundingClientRect(),
    mx = e.clientX - rect.left,
    my = e.clientY - rect.top;
  const v = f.views[Number($("fog").value)];
  // Locate the diamond containing the pointer, including staggered rows.
  let hit;
  for (
    let row = Math.max(0, Math.floor(my / (tileH / 2)) - 1);
    row <= Math.min(f.height - 1, Math.floor(my / (tileH / 2)));
    row++
  ) {
    const col = Math.floor((mx - ((row % 2) * tileW) / 2) / tileW),
      x = col * 2 + (row % 2);
    if (
      col >= 0 &&
      col < f.width &&
      Math.abs(mx - ((x + 1) * tileW) / 2) / (tileW / 2) +
        Math.abs(my - ((row + 1) * tileH) / 2) / (tileH / 2) <=
        1
    )
      hit = { x, y: row, i: row * f.width + col };
  }
  if (!hit) return;
  const t = v.tiles[hit.i],
    city = v.cities.find((c) => c.x === hit.x && c.y === hit.y),
    units = v.units.filter((u) => u.x === hit.x && u.y === hit.y);
  $("details").textContent =
    `${hit.x},${hit.y} · ${t ? (f.terrain_names?.[t[0]] || `Terrain ${t[0]}`) + (t[4] ? " (special suppressed)" : "") : "Unexplored"}${city ? ` · ${city.name} (civ ${city.owner}, population ${city.size})` : ""}${units.length ? ` · Units: ${units.map((u) => `${f.unit_names?.[u.type] || u.type} / civ ${u.owner}${u.veteran ? " veteran" : ""}`).join(", ")}` : ""}`;
};
function validate(f) {
  if (
    !f ||
    f.schema !== 1 ||
    !Number.isInteger(f.turn) ||
    !Number.isInteger(f.year) ||
    typeof f.checksum !== "string" ||
    !/^[0-9a-f]{16}$/.test(f.checksum) ||
    !Number.isInteger(f.width) ||
    !Number.isInteger(f.height) ||
    f.width < 1 ||
    f.height < 1 ||
    f.width * f.height > 2000000 ||
    !Array.isArray(f.views) ||
    f.views.length !== 8 ||
    !Array.isArray(f.events) ||
    !f.events.every((e) => typeof e === "string")
  )
    throw Error("Unsupported replay frame");
  for (const [i, v] of f.views.entries()) {
    if (
      !v ||
      v.civ !== (i || null) ||
      !Array.isArray(v.tiles) ||
      v.tiles.length !== f.width * f.height ||
      !v.tiles.every(
        (t) =>
          t === null ||
          (Array.isArray(t) &&
            t.length === 5 &&
            Number.isInteger(t[0]) &&
            t[0] >= 0 &&
            t[0] <= 15 &&
            t.slice(1).every((b) => typeof b === "boolean")),
      ) ||
      !Array.isArray(v.units) ||
      !Array.isArray(v.cities)
    )
      throw Error("Invalid replay view");
    for (const o of [...v.units, ...v.cities])
      if (
        !Number.isFinite(o.x) ||
        !Number.isFinite(o.y) ||
        !Number.isInteger(o.owner)
      )
        throw Error("Invalid replay object");
  }
  return f;
}
$("file").onchange = async () => {
  pause();
  if (busy) {
    fail(
      Error("Wait for the current turn to finish, then open the replay again."),
    );
    return;
  }
  busy = true;
  try {
    const file = $("file").files[0];
    if (!file) return;
    const loaded = (await file.text())
      .trim()
      .split("\n")
      .map((line) => validate(JSON.parse(line)));
    if (
      loaded.some(
        (f, i) =>
          i &&
          (f.turn !== loaded[i - 1].turn + 1 ||
            f.width !== loaded[0].width ||
            f.height !== loaded[0].height),
      )
    )
      throw Error(
        "Replay frames must have consecutive turns and matching map dimensions",
      );
    frames = loaded;
    total = loaded.length;
    offline = true;
    index = target = 0;
    $("error").textContent = "";
    draw();
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
  }
};
$("download").onclick = () => {
  const blob = new Blob(
      [frames.map((f) => JSON.stringify(f)).join("\n") + "\n"],
      { type: "application/x-ndjson" },
    ),
    url = URL.createObjectURL(blob),
    a = document.createElement("a");
  a.href = url;
  a.download = "totengine-replay.ndjson";
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
};
(async () => {
  try {
    const status = await json("/status");
    total = status.total;
    await go(0);
  } catch (e) {
    fail(e);
  }
})();
