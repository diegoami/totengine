// Optional real-browser smoke test. Install Playwright outside the repo and
// set NODE_PATH to that node_modules directory. Start `totctl observe` with
// --turns 100, then: node tools/test_observer_browser.cjs http://127.0.0.1:PORT
const { chromium } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
(async () => {
  const browser = await chromium.launch({ headless: true });
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "totengine-browser-"));
  try {
    const page = await browser.newPage({
      viewport: { width: 1280, height: 900 },
    });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(process.argv[2] || "http://127.0.0.1:8000");
    await page.waitForFunction(() =>
      document.getElementById("status").textContent.includes("1/101"),
    );
    const boot = await page.locator("#status").textContent();
    const initialPixels = await page
      .locator("#map")
      .evaluate((c) => c.toDataURL());
    await page.locator("#fog").selectOption("1");
    assert.notEqual(
      await page.locator("#map").evaluate((c) => c.toDataURL()),
      initialPixels,
      "fog changes rendered tiles",
    );
    await page.locator("#fog").selectOption("0");
    await page.locator("#next").click();
    await page.waitForFunction(
      () => document.getElementById("timeline").value === "1",
    );
    await page.locator("#back").click();
    await page.waitForFunction(
      () => document.getElementById("timeline").value === "0",
    );
    assert.equal(await page.locator("#status").textContent(), boot);
    await page.locator("#speed").selectOption("250");
    await page.locator("#play").click();
    await page.waitForFunction(
      () => Number(document.getElementById("timeline").value) >= 3,
    );
    await page.locator("#play").click();
    await page.waitForTimeout(500);
    const paused = await page.locator("#timeline").inputValue();
    await page.waitForTimeout(500);
    assert.equal(
      await page.locator("#timeline").inputValue(),
      paused,
      "pause stops advancing",
    );
    // Hold one uncached frame to exercise a second seek while the first
    // is awaiting the server. Restart must stop further generation.
    let releaseFrame, requestedFrame;
    const held = new Promise((resolve) => {
      releaseFrame = resolve;
    });
    const requested = new Promise((resolve) => {
      requestedFrame = resolve;
    });
    await page.route("**/frame/10", async (route) => {
      requestedFrame();
      await held;
      await route.continue();
    });
    await page.locator("#timeline").evaluate((el) => {
      el.value = "100";
      el.dispatchEvent(new Event("input"));
    });
    await requested;
    await page.locator("#restart").click();
    releaseFrame();
    await page.waitForFunction(() => !busy);
    assert.equal(
      await page.locator("#status").textContent(),
      boot,
      "restart cancels an in-progress seek",
    );
    const cached = await page.evaluate(() => frames.length);
    await page.waitForTimeout(300);
    assert.equal(
      await page.evaluate(() => frames.length),
      cached,
      "cancelled seek stops new requests",
    );
    await page.locator("#timeline").evaluate((el) => {
      el.value = "100";
      el.dispatchEvent(new Event("input"));
    });
    await page.waitForFunction(
      () => document.getElementById("status").textContent.includes("101/101"),
      null,
      { timeout: 60000 },
    );
    assert.equal(await page.locator("#next").isDisabled(), true);
    const final = await page.locator("#status").textContent();
    const finalPixels = await page
      .locator("#map")
      .evaluate((c) => c.toDataURL());
    await page.screenshot({ path: path.join(dir, "observer.png") });
    const downloadPromise = page.waitForEvent("download");
    await page.locator("#download").click();
    const download = await downloadPromise;
    const replay = path.join(dir, "replay.ndjson");
    await download.saveAs(replay);
    const data = (await fs.readFile(replay, "utf8"))
      .trim()
      .split("\n")
      .map(JSON.parse);
    assert.equal(data.length, 101);
    assert(
      data[0].views[0].tiles.every(
        (t) => t.length === 5 && typeof t[4] === "boolean",
      ),
      "special-suppression flag is archived",
    );
    assert.equal(data[100].turn - data[0].turn, 100);
    await page.locator("#restart").click();
    await page.waitForFunction(
      () => document.getElementById("timeline").value === "0",
    );
    await page.locator("#file").setInputFiles(replay);
    await page.waitForFunction(() => offline);
    await page.locator("#timeline").evaluate((el) => {
      el.value = "100";
      el.dispatchEvent(new Event("input"));
    });
    await page.waitForFunction(() =>
      document.getElementById("status").textContent.includes("101/101"),
    );
    assert.equal(await page.locator("#status").textContent(), final);
    assert.equal(
      await page.locator("#map").evaluate((c) => c.toDataURL()),
      finalPixels,
      "archive playback renders identical pixels",
    );
    assert.equal(await page.locator("#error").textContent(), "");
    assert.equal(
      await page.locator("#orderlog").isVisible(),
      false,
      "archive playback hides the unrelated live order log",
    );
    const badReplay = path.join(dir, "bad.ndjson");
    await fs.writeFile(
      badReplay,
      JSON.stringify({ ...data[0], schema: 999 }) + "\n",
    );
    await page.locator("#file").setInputFiles(badReplay);
    await page.waitForFunction(
      () => document.getElementById("error").textContent !== "",
    );
    assert.equal(
      await page.locator("#status").textContent(),
      final,
      "bad import preserves the current replay",
    );
    assert.deepEqual(errors, []);
    console.log(
      `PASS: live 100 turns, fog, step/back, play/pause, restart during seek, download/import, invalid archive rejection, identical replay pixels. Screenshot: ${path.join(dir, "observer.png")}`,
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
