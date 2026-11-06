// Optional real-browser check; install Playwright outside the repo and set NODE_PATH.
// Start tools/preview_client.py, then:
// node tools/test_client_artwork.cjs http://127.0.0.1:8765 /tmp/client-artwork.json
// The optional report contains numeric metadata only. No artwork/screenshots written.
const { chromium } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({
      viewport: { width: 1440, height: 1100 },
    });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(process.argv[2] || "http://127.0.0.1:8765");
    await page.waitForFunction(
      () =>
        window.installedArtwork ||
        document.querySelector("#artwork-status")?.dataset.error,
    );
    const metadata = await page.evaluate(() => window.installedArtwork);
    assert.ok(metadata, await page.locator("#artwork-status").textContent());
    for (const [name, sample] of Object.entries(metadata.measurements)) {
      assert.ok(
        sample && sample.opaque > 0 && sample.transparent > 0,
        `${name} has foreground and transparency`,
      );
      assert.equal(
        sample.visibleMagenta,
        0,
        `${name} has no visible magenta key`,
      );
      assert.equal(sample.cornerAlpha, 0, `${name} corner background cleared`);
    }
    // Confirm the browser consumed the bytes identified by the local source manifest.
    for (const asset of Object.values(metadata.sources)) {
      const actual = await page.evaluate(async (url) => {
        const bytes = await (await fetch(url)).arrayBuffer();
        return Array.from(
          new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
        )
          .map((x) => x.toString(16).padStart(2, "0"))
          .join("");
      }, asset.url);
      assert.equal(actual, asset.sha256);
    }
    const screen = async (name) =>
      page
        .getByRole("navigation", { name: "Review screens" })
        .getByRole("button", { name, exact: true })
        .click();
    const initialPixels = await page
      .locator("#mapcanvas")
      .evaluate((c) => c.toDataURL());
    await page.locator("#zoomin").click();
    assert.notEqual(
      await page.locator("#mapcanvas").evaluate((c) => c.toDataURL()),
      initialPixels,
    );
    await page.locator("#zoomout").click();
    await page.locator("#mode").click();
    await page.locator("#movestyle").selectOption("direct");
    const [x, y] = await page.evaluate(() => mapPoint(8, 5));
    const box = await page.locator("#mapcanvas").boundingBox();
    await page
      .locator("#mapcanvas")
      .click({ position: { x: (x * box.width) / 1380, y } });
    assert.deepEqual(await page.evaluate(() => state.unit), [8, 5]);
    await screen("City");
    assert.equal(await page.locator(".citizens img").count(), 1);
    assert.equal(await page.locator(".resources img").count(), 6);
    const before = await page.locator(".citizens img").getAttribute("src");
    await page.locator("#specialist").click();
    assert.notEqual(
      await page.locator(".citizens img").getAttribute("src"),
      before,
    );
    assert.equal(
      await page.locator(".citizens img").count(),
      1,
      "no duplicate adornments on redraw",
    );
    assert.equal(await page.locator("#endturn").isDisabled(), true);
    for (const name of [
      "Map",
      "City",
      "Production",
      "Research",
      "Government",
      "Diplomacy",
      "Reports",
      "Setup",
      "Save / load",
    ]) {
      await screen(name);
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        ),
        false,
        `desktop overflow: ${name}`,
      );
    }
    await page.setViewportSize({ width: 1024, height: 768 });
    await screen("City");
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
      "city at 1024px",
    );
    assert.deepEqual(errors, []);
    if (process.argv[3])
      await fs.writeFile(
        process.argv[3],
        JSON.stringify(
          {
            schema: 1,
            kind: "client-artwork-preview",
            ...metadata,
            checks:
              "source hashes, sample masks, zoom, move, citizen redraw, nine screens, 1440/1024 layout, no JS errors",
          },
          null,
          2,
        ) + "\n",
      );
    console.log(
      `PASS installed artwork: ${metadata.profile}; source hashes, sample masks and browser flows.`,
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
