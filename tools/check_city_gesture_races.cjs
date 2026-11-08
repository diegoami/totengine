// Chromium regressions for #166–#168. Inputs use the real event listeners;
// routing controls POST completion, and only the 20ms opponent timer is held.
const fs = require('fs'), path = require('path');
exports.run = async ({page, state, key, click, save, load, idle, fixture, saves}) => {
  fs.copyFileSync(fixture, path.join(saves, 'cityrace.cth'));
  const isCity = f => f.draws.some(d => d.kind === 'text' && d.x === 18 && d.y === 14 && d.text.includes('population'));
  async function map() {
    await load('cityrace');
    if (isCity(await state())) await key('Escape');
    await idle();
    const d = (await state()).draws.find(d => d.kind === 'sprite' && d.sprite.startsWith('city-'));
    const r = await page.locator('#game').boundingBox();
    return [r.x + (d.x + d.w / 2) * r.width / 1024, r.y + (d.y + d.h * .6) * r.height / 768];
  }
  async function checkpoint(name) {
    await key('Control+s'); await save(name);
    if ((await state()).status !== 'Engine checkpoint saved: ' + name) throw Error('checkpoint was not saved: ' + name);
    return fs.readFileSync(path.join(saves, name + '.cth'));
  }
  async function heldPost(type) {
    let release, started, captured = false;
    const held = new Promise(r => release = r), ready = new Promise(r => started = r);
    const handler = async route => {
      if (captured || route.request().postDataJSON()?.input?.type !== type) return route.fallback();
      captured = true;
      const response = await route.fetch();
      started(); await held; await route.fulfill({response});
    };
    await page.route('**/input', handler);
    return {ready, release};
  }
  for (const newer of ['key', 'click']) {
    const [x, y] = await map();
    const first = await heldPost('click');
    await page.mouse.dblclick(x, y, {delay: 30}); await first.ready;
    if (!await page.evaluate(() => pendingDoubleClick !== null)) throw Error('second click was not queued');
    if (newer === 'key') await page.keyboard.press('v');
    else await page.mouse.click(x + 110, y);
    if (await page.evaluate(() => pendingDoubleClick !== null)) throw Error('newer ' + newer + ' retained older gesture');
    first.release(); await idle();
    if (isCity(await state())) throw Error('older gesture opened city after newer ' + newer);
    // The first garrison click can legitimately leave its unit chooser open.
    await key('Escape');
    if (!(await checkpoint('cityrace-' + newer)).equals(fs.readFileSync(fixture))) throw Error('busy intent changed gameplay');
  }
  // Reject an out-of-range second click; a detail-3 event at the original
  // point must not reuse that first ID. No intervening key masks invalidation.
  const [x, y] = await map();
  await page.mouse.click(x, y); await idle();
  await page.evaluate(([x, y]) => canvas.dispatchEvent(new MouseEvent('click', {clientX:x + 100, clientY:y, detail:2})), [x,y]);
  await idle();
  if ((await state()).city_click !== null) throw Error('invalid second click retained gesture');
  await page.evaluate(([x, y]) => canvas.dispatchEvent(new MouseEvent('click', {clientX:x, clientY:y, detail:3})), [x,y]);
  await idle();
  if (isCity(await state())) throw Error('third event repaired invalid second click');
  await key('Escape');
  if (!(await checkpoint('cityrace-invalid')).equals(fs.readFileSync(fixture))) throw Error('invalid gesture changed gameplay');

  async function endTurn() {
    await key('Control+n');
    if ((await state()).draws.some(d => d.text === 'End your turn with movable units?')) await key('Enter');
  }
  async function finishTurn() {
    await page.waitForFunction(() => frame.phase !== 'Opponents' && !busy);
    if (isCity(await state())) await key('Escape');
    if ((await state()).draws.some(d => d.text === 'Choose research explicitly')) {
      await key('ArrowDown'); await key('Enter');
    }
  }
  await map(); await endTurn(); await finishTurn();
  const expected = await checkpoint('cityrace-next-expected');
  const p = await map();
  await page.evaluate(() => {
    window.cityRaceTimers = [];
    window.cityRaceSetTimeout = window.setTimeout;
    window.setTimeout = (fn, ms, ...args) => ms === 20
      ? (cityRaceTimers.push(() => fn(...args)), -cityRaceTimers.length)
      : cityRaceSetTimeout(fn, ms, ...args);
  });
  try {
    await endTurn();
    if ((await state()).phase !== 'Opponents') throw Error('opponent race did not enter Opponents');
    const first = await heldPost('click'), second = await heldPost('double_click');
    await page.mouse.dblclick(...p, {delay:30}); await first.ready;
    first.release(); await second.ready;
    // Fire the sole scheduled tick while the second POST is busy. The final
    // double-click response must arrange another tick without any user input.
    if (!await page.evaluate(() => { const fn = cityRaceTimers.shift(); if (!fn) return false; fn(); return busy; }))
      throw Error('opponent timer was not held across the double click');
    second.release(); await idle();
    for (let n = 0; n < 200 && (await state()).phase === 'Opponents'; n++) {
      if (!await page.evaluate(() => { const fn = cityRaceTimers.shift(); if (!fn) return false; fn(); return true; }))
        throw Error('opponent progression lost its continuation tick');
      await idle();
    }
    if ((await state()).phase === 'Opponents') throw Error('opponents did not complete');
  } finally {
    await page.evaluate(() => { window.setTimeout = cityRaceSetTimeout; for (const fn of cityRaceTimers.splice(0)) fn(); });
  }
  await finishTurn();
  if (!(await checkpoint('cityrace-next')).equals(expected)) throw Error('raced opponent round differs from ordinary round');
  // Fail both retry attempts before reaching the host. Only periodic /state
  // recovery may resume the round; do not send any user input while waiting.
  await map();
  let aborted = 0, refreshed = 0, recoveredTick = 0;
  const attempts = [];
  const reconnect = async route => {
    const request = route.request();
    if (request.url().endsWith('/input') && request.postDataJSON()?.input?.type === 'tick') {
      if (aborted < 2) {
        attempts.push(request.postDataJSON()); aborted++;
        return route.abort('failed');
      }
      recoveredTick++;
    }
    if (request.url().endsWith('/state') && aborted === 2) {
      const response = await route.fetch();
      if (response.ok()) refreshed++;
      return route.fulfill({response});
    }
    return route.fallback();
  };
  await page.route('**/*', reconnect);
  try {
    await endTurn();
    await page.waitForFunction(() => frame.phase !== 'Opponents' && !busy, null, {timeout:10000});
    if (aborted !== 2 || refreshed < 1 || recoveredTick < 1) throw Error('reconnect did not recover through periodic state refresh');
    if (JSON.stringify(attempts[0]) !== JSON.stringify(attempts[1])) throw Error('failed tick retry changed identity or payload');
  } finally { await page.unroute('**/*', reconnect); }
  await finishTurn();
  if (!(await checkpoint('cityrace-reconnect')).equals(expected)) throw Error('reconnected next round differs from ordinary round');
  console.log(JSON.stringify({city_gesture_races:'PASS', checkpoints:6, cases:['busy-new-key','busy-new-click','invalid-second-third','opponent-double-post-tick','failed-tick-reconnect']}));
};
