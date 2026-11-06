/* Local design-only artwork renderer. Reads images from preview_client.py;
 * no assets or rendered derivatives are embedded or written to the repository.
 * Crop recipes are measured examples, not a complete ToT atlas/rendering spec. */
(async () => {
  if (!/^https?:$/.test(location.protocol)) return;
  const banner = document.createElement("p");
  banner.id = "artwork-status";
  banner.textContent = "Loading installed artwork…";
  document.querySelector(".review > div").append(banner);
  try {
    const response = await fetch("/assets/manifest");
    if (!response.ok)
      throw new Error(
        "Artwork manifest unavailable. Start tools/preview_client.py.",
      );
    const manifest = await response.json();
    const sheets = {};
    await Promise.all(
      Object.entries(manifest.assets).map(async ([key, asset]) => {
        const image = new Image();
        image.src = asset.url;
        await image.decode();
        sheets[key] = image;
      }),
    );
    const crops = new Map();
    function crop(
      key,
      x,
      y,
      w,
      h,
      { diamond = false, unit = false, matte = false, pink = false } = {},
    ) {
      const id = [key, x, y, w, h, diamond, unit, matte, pink].join(":");
      if (crops.has(id)) return crops.get(id);
      const source = sheets[key];
      if (!source || x + w > source.width || y + h > source.height) return null;
      const canvas = document.createElement("canvas");
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(source, x, y, w, h, 0, 0, w, h);
      const data = ctx.getImageData(0, 0, w, h);
      const pinkKey = pink ? Array.from(data.data.slice(0, 3)) : null;
      for (let py = 0; py < h; py++)
        for (let px = 0; px < w; px++) {
          const i = (py * w + px) * 4;
          const [r, g, b] = data.data.slice(i, i + 3);
          const outside =
            diamond &&
            Math.abs((px + 0.5 - w / 2) / (w / 2)) +
              Math.abs((py + 0.5 - h / 2) / (h / 2)) >
              1;
          if (
            outside ||
            (r === 255 && g === 0 && b === 255) ||
            (unit && r === 132 && g === 82 && b === 132)
          )
            data.data[i + 3] = 0;
        }
      if (matte) {
        const visited = new Uint8Array(w * h),
          queue = [];
        function visit(px, py) {
          if (px < 0 || py < 0 || px >= w || py >= h) return;
          const pos = py * w + px,
            i = pos * 4;
          if (visited[pos]) return;
          visited[pos] = 1;
          const gray =
            data.data[i] === 132 &&
            data.data[i + 1] === 132 &&
            data.data[i + 2] === 132;
          if (
            gray ||
            (pink &&
              data.data[i] === pinkKey[0] &&
              data.data[i + 1] === pinkKey[1] &&
              data.data[i + 2] === pinkKey[2]) ||
            data.data[i + 3] === 0
          ) {
            data.data[i + 3] = 0;
            queue.push(pos);
          }
        }
        for (let x = 0; x < w; x++) {
          visit(x, 0);
          visit(x, h - 1);
        }
        for (let y = 0; y < h; y++) {
          visit(0, y);
          visit(w - 1, y);
        }
        for (let head = 0; head < queue.length; head++) {
          const pos = queue[head],
            x = pos % w,
            y = Math.floor(pos / w);
          visit(x - 1, y);
          visit(x + 1, y);
          visit(x, y - 1);
          visit(x, y + 1);
        }
      }
      ctx.putImageData(data, 0, 0);
      crops.set(id, canvas);
      return canvas;
    }
    function terrain(id, variant = 0) {
      return crop("terrain1", 1 + (variant % 3) * 65, 1 + id * 33, 64, 32, {
        diamond: true,
      });
    }
    function overlay(row, variant = 0) {
      return crop("terrain2", 1 + (variant % 8) * 65, 1 + row * 33, 64, 32, {
        matte: true,
      });
    }
    function unit(index) {
      return crop(
        "units",
        1 + (index % 9) * 65,
        1 + Math.floor(index / 9) * 65,
        64,
        64,
        { unit: true, matte: true },
      );
    }
    function city(style = 1, size = 0) {
      return crop("cities", 1 + size * 65, 39 + style * 49, 64, 48, {
        matte: true,
      });
    }
    function blit(ctx, image, x, y, w, h) {
      if (image) ctx.drawImage(image, x, y, w, h);
    }
    const texture = crop("dialog", 1, 1, 92, 92);
    const panel = crop("city", 430, 20, 110, 110);
    if (texture || panel) {
      const style = document.createElement("style");
      style.textContent = `
        .client { border: 4px ridge #a6a69a; --gold: #eee193; --panel: #202021; }
        .client .statusbar { background: #65675f; }
        .client .statusbar>div,.client .box,.client .orderbar { background-color: #202021; ${panel ? `background-image:linear-gradient(#0008,#0008),url('${panel.toDataURL()}');` : ""} }
        .client .screen { background-color: #15191a; }
        .client button { font-family: Georgia,serif; border: 3px ridge #aaa99b; color:#eee; ${texture ? `background-image:url('${texture.toDataURL()}');` : ""} }
        .client button.primary,.client button[aria-pressed=true] { color:#fff6b5; outline-color:#e8d88a; }
        .client .box { border: 3px ridge #969b8b; }
        .client .citizens img { width:27px; height:32px; image-rendering:pixelated; vertical-align:middle; }
        .client .asset-unit { width:48px; height:48px; object-fit:contain; image-rendering:pixelated; vertical-align:middle; }
        .client .asset-people { display:flex; gap:4px; margin:10px 0; }
        .client .asset-people img { width:27px; height:32px; image-rendering:pixelated; }
        .client canvas { image-rendering:pixelated; }
      `;
      document.head.append(style);
    }
    function fixtureTerrain(x, y) {
      if (
        x === 3 ||
        x === 14 ||
        y === 1 ||
        y === 9 ||
        (x < 6 && y < 4) ||
        (x > 12 && y > 7)
      )
        return 10;
      if (y === 3 && x >= 6 && x <= 9) return 5;
      if (y === 4 && x >= 6 && x <= 8) return 4;
      if (x >= 10 && x <= 12 && y >= 4 && y <= 6) return 3;
      if (y >= 7 && x < 8) return 1;
      return 2;
    }
    function paintTile(ctx, x, y, px, py, scale, id) {
      blit(
        ctx,
        terrain(id, (x + y) % 3),
        px - 32 * scale,
        py - 16 * scale,
        64 * scale,
        32 * scale,
      );
      if (id === 3 || id === 4 || id === 5) {
        const row = { 3: 4, 4: 8, 5: 6 }[id];
        blit(
          ctx,
          overlay(row, (x + y) % 8),
          px - 32 * scale,
          py - 16 * scale,
          64 * scale,
          32 * scale,
        );
      }
    }
    mapPoint = (x, y) => [
      560 + (x - y) * 37 * state.zoom,
      95 + (x + y) * 18.5 * state.zoom,
    ];
    drawMap = function () {
      const c = $("mapcanvas");
      if (!c) return;
      const ctx = c.getContext("2d");
      ctx.imageSmoothingEnabled = false;
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, c.width, c.height);
      // Keep the authored tile coordinates/hit testing; only change presentation.
      const scale = (74 * state.zoom) / 64;
      for (let y = 1; y <= 9; y++)
        for (let x = 3; x <= 14; x++) {
          const [px, py] = mapPoint(x, y);
          paintTile(ctx, x, y, px, py, scale, fixtureTerrain(x, y));
        }
      for (const [x, y, name, size] of [
        [7, 5, "Alder", 0],
        [12, 4, "Riverwatch", 1],
      ]) {
        const [px, py] = mapPoint(x, y);
        blit(
          ctx,
          city(1, size),
          px - 32 * scale,
          py - 34 * scale,
          64 * scale,
          48 * scale,
        );
        ctx.font = "bold 19px Georgia";
        ctx.textAlign = "center";
        ctx.strokeStyle = "#000";
        ctx.lineWidth = 3;
        ctx.strokeText(name, px, py + 34 * scale);
        ctx.fillStyle = "#ffef81";
        ctx.fillText(name, px, py + 34 * scale);
      }
      const [ux, uy] = mapPoint(...state.unit);
      blit(
        ctx,
        unit(2),
        ux - 32 * scale,
        uy - 48 * scale,
        64 * scale,
        64 * scale,
      );
      ctx.strokeStyle = "#fff1a0";
      ctx.lineWidth = 1;
      ctx.strokeRect(ux - 25 * scale, uy - 43 * scale, 50 * scale, 56 * scale);
      ctx.fillStyle = "#6eca4c";
      ctx.fillRect(ux - 18 * scale, uy - 46 * scale, 30 * scale, 3 * scale);
      let mini = document.querySelector(".minimap canvas");
      if (!mini) {
        mini = document.createElement("canvas");
        mini.width = 122;
        mini.height = 64;
        mini.setAttribute(
          "aria-label",
          "Authored map overview using installed artwork",
        );
        document.querySelector(".minimap svg").replaceWith(mini);
      }
      mini
        .getContext("2d")
        .drawImage(c, 0, 0, c.width, c.height, 0, 0, 122, 64);
    };
    drawCity = function () {
      const c = $("citycanvas");
      if (!c) return;
      const ctx = c.getContext("2d");
      ctx.imageSmoothingEnabled = false;
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, c.width, c.height);
      let k = 0;
      const scale = 104 / 64;
      for (let y = -2; y <= 2; y++)
        for (let x = -2; x <= 2; x++) {
          if (Math.abs(x) === 2 && Math.abs(y) === 2) continue;
          const px = 325 + (x - y) * 52,
            py = 150 + (x + y) * 26;
          const id =
            x === -1 && y < 0 ? 5 : x > 0 && y < 0 ? 3 : y === 2 ? 10 : 2;
          paintTile(ctx, x + 4, y + 4, px, py, scale, id);
          if (k++ < state.worked) {
            ctx.fillStyle = "#fff094";
            ctx.font = "bold 19px Georgia";
            ctx.fillText("◆", px - 7, py + 6);
          }
          if (x === 0 && y === 0)
            blit(
              ctx,
              city(1, 1),
              px - 32 * scale,
              py - 34 * scale,
              64 * scale,
              48 * scale,
            );
        }
    };
    function decorate() {
      if (state.screen === "map") drawMap();
      if (state.screen === "city") {
        drawCity();
        const icons = [
          [1, 224],
          [22, 245],
          [1, 245],
          [43, 245],
          [22, 224],
          [43, 203],
        ];
        document.querySelectorAll(".resources dt").forEach((label, index) => {
          const [x, y] = icons[index],
            icon = crop("icons", x, y, 20, 20, { matte: true, pink: true });
          if (icon) {
            const image = document.createElement("img");
            image.src = icon.toDataURL();
            image.alt = "";
            image.width = 18;
            image.height = 18;
            image.style.verticalAlign = "middle";
            image.style.marginRight = "6px";
            label.prepend(image);
          }
        });
        const role = $("specialist");
        const face = crop(
          "people",
          1 + [2, 4, 9, 10][state.citizens] * 28,
          1,
          27,
          32,
        );
        if (role && face) {
          const image = document.createElement("img");
          image.src = face.toDataURL();
          image.alt = "";
          role.prepend(image);
        }
        const item = document.querySelector(
          ".citygrid .stack .box:nth-child(2) h3",
        );
        const itemIndex = { Settlers: 0, Warriors: 2, Phalanx: 3 }[
          state.production
        ];
        if (item && itemIndex !== undefined) {
          const image = document.createElement("img");
          image.className = "asset-unit";
          image.src = unit(itemIndex).toDataURL();
          image.alt = "";
          item.prepend(image);
        }
      }
    }
    const originalShow = show;
    show = function (...args) {
      originalShow(...args);
      decorate();
    };
    banner.textContent = `Installed artwork · ${manifest.profile} · simulated game state`;
    document.querySelector(".review > div p").textContent =
      "Proposed client layouts using your installed artwork. Actions and values are simulated.";
    const note = document.querySelectorAll(".review > div p")[1];
    note.replaceChildren(
      document.createTextNode("No engine connection or real save support. "),
    );
    const link = document.createElement("a");
    link.href = "/client-design.md";
    link.textContent = "Design notes";
    note.append(link);
    document.querySelector(".footer").textContent =
      "Local artwork preview: assets are read from your installation. Fixture geography, transparency/crop recipes and terrain adjacency are provisional; this is not a native rendering or gameplay parity claim. Screenshots containing this artwork stay outside Git.";
    // Expose dimensions/source hashes for repeatable numeric evidence, not pixels.
    function measure(image) {
      if (!image) return null;
      const rgba = image
        .getContext("2d")
        .getImageData(0, 0, image.width, image.height).data;
      let opaque = 0,
        transparent = 0,
        magenta = 0;
      for (let i = 0; i < rgba.length; i += 4) {
        if (rgba[i + 3] === 0) transparent++;
        else {
          opaque++;
          if (rgba[i] === 255 && rgba[i + 1] === 0 && rgba[i + 2] === 255)
            magenta++;
        }
      }
      return {
        width: image.width,
        height: image.height,
        opaque,
        transparent,
        visibleMagenta: magenta,
        cornerAlpha: rgba[3],
      };
    }
    window.installedArtwork = {
      profile: manifest.profile,
      measurements: {
        foodIcon: measure(
          crop("icons", 1, 224, 20, 20, { matte: true, pink: true }),
        ),
        grass: measure(terrain(2)),
        forest: measure(overlay(4)),
        warrior: measure(unit(2)),
        city: measure(city(1)),
      },
      sources: manifest.assets,
      dimensions: Object.fromEntries(
        Object.keys(manifest.assets).map((k) => [
          k,
          [sheets[k].width, sheets[k].height],
        ]),
      ),
    };
    decorate();
  } catch (error) {
    banner.textContent = `Installed artwork unavailable: ${error.message}`;
    banner.dataset.error = "true";
  }
})();
