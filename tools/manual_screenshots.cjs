// Capture the user guide's screenshots from the real browser client.
//
// Spawns totplay (Original profile via CIVTOT_HOME), walks the guide's
// sections in a fresh generated game, and writes PNGs into
// docs/manual/screenshots. Every shot is wrapped independently so one
// failure does not lose the rest; the game-start path itself asserts.
//
// Env: CIVTOT_HOME (install), TOTENGINE_BROWSER_EXECUTABLE (chromium),
// NODE_PATH must resolve the playwright module.
const {chromium}=require('playwright');
const {spawn}=require('child_process');
const fs=require('fs');
const path=require('path');

const origin='http://127.0.0.1:8773';
const repo=path.resolve(__dirname,'..');
const out=path.join(repo,'docs','manual','screenshots');
fs.mkdirSync(out,{recursive:true});
const savesDir='/tmp/manual-screenshots-saves';
fs.rmSync(savesDir,{recursive:true,force:true});
fs.mkdirSync(savesDir,{recursive:true});
const totplay=path.resolve(process.env.TOTENGINE_TOTPLAY
  ||path.join(repo,'..','totengine-m17-turn48-audit-target','debug','totplay'));

const server=spawn(totplay,['--no-open','--port=8773'],{
  env:{...process.env,CIVTOT_HOME:process.env.CIVTOT_HOME,
       TOTENGINE_SAVES:savesDir},
  stdio:['ignore','pipe','pipe']});
server.stdout.on('data',d=>process.stdout.write('[totplay] '+d));
server.stderr.on('data',d=>process.stderr.write('[totplay] '+d));

async function waitForServer(){
  for(let n=0;n<60;n++){
    try{const r=await fetch(origin+'/state');if(r.ok)return}catch{}
    await new Promise(r=>setTimeout(r,500));
  }
  throw Error('totplay did not serve '+origin);
}

(async()=>{
  await waitForServer();
  const browser=await chromium.launch({headless:true,args:['--no-sandbox'],
    executablePath:process.env.TOTENGINE_BROWSER_EXECUTABLE});
  const page=await browser.newPage({viewport:{width:1024,height:768}});
  page.on('pageerror',e=>console.error('PAGEERROR',e));
  await page.goto(origin);
  async function idle(){await page.waitForFunction(()=>typeof frame!=='undefined'&&frame&&!busy&&paintedRevision===frame.revision)}
  async function state(){await idle();return page.evaluate(()=>frame)}
  async function resizeViewport(width,height){
    const display=page.waitForResponse(r=>r.url().endsWith('/input')&&r.request().postDataJSON()?.input?.type==='display');
    await page.setViewportSize({width,height});
    await display;
    const wanted=Math.min(1792,Math.max(1024,Math.floor(width*768/height)));
    await page.waitForFunction(w=>frame?.width===w&&!busy&&!pendingDisplay&&paintedRevision===frame.revision,wanted);
  }
  async function key(k){await idle();await page.keyboard.press(k);await idle()}
  async function click(label){
    const f=await state(),t=f.draws.find(d=>d.kind==='text'&&d.text===label);
    if(!t)throw Error('Missing '+label);
    const rect=await page.locator('#game').boundingBox();
    await page.mouse.click(rect.x+(t.x+18)*rect.width/f.width,rect.y+(t.y+9)*rect.height/768);
    await idle();
  }
  async function fill(v){await idle();await page.locator('#entry').fill(v);await page.waitForFunction(x=>frame.entry?.value===x&&!busy,v);await idle()}
  async function save(n){await fill(n);await key('Enter')}
  async function shot(name){await idle();await page.screenshot({path:path.join(out,name+'.png')});console.log('captured',name)}
  function has(f,t){return f.draws.some(d=>d.kind==='text'&&d.text===t)}
  async function research(){
    if(has(await state(),'Choose research')){
      await key('ArrowDown');await key('Enter');
      await page.waitForFunction(()=>frame.status==='research-set',{timeout:20000}).catch(async()=>{
        const f=await state();
        console.error('research-set timeout; draws:',JSON.stringify(f.draws.filter(d=>d.kind==='text').map(d=>d.text).slice(0,12)));
        throw Error('research chooser did not settle');
      });
    }
  }

  await page.evaluate(()=>dispatchEvent(new Event('focus')));
  await page.waitForTimeout(300);await idle();

  // Setup screen.
  await shot('setup');

  // Experimental scenario preflight (the installed-save listing).
  try{
    await click('Open scenario (experimental)');
    await page.waitForTimeout(500);await idle();
    await shot('scenario-preflight');
    await key('Escape');await key('Escape');
  }catch(e){console.error('scenario-preflight failed:',e.message)}

  // Start a fresh Original-rules game, found Athens, choose production.
  await click('Generated seed');await fill('7');await click('Start game (Original rules)');
  if(has(await state(),'Save pending choice')){await click('Save pending choice');await save('research')}
  await research();
  await key('b');
  if(has(await state(),'Save pending choice')){
    await click('Save pending choice');await fill('founding');await key('Enter');
  }
  await fill('athena');await key('Enter');
  const afterName=await state();
  console.log('after name:',JSON.stringify({status:afterName.status,
    texts:afterName.draws.filter(d=>d.kind==='text').map(d=>d.text).slice(0,14)}));
  // The original's route opens the city from its map tile (decision 0009,
  // #439); production is chosen inside the city window.
  try{
    let f=await state();
    let city=f.draws.find(d=>d.kind==='sprite'&&d.sprite&&d.sprite.startsWith('city-'));
    if(!city){await key('c');f=await state();city=f.draws.find(d=>d.kind==='sprite'&&d.sprite&&d.sprite.startsWith('city-'))}
    const rect=await page.locator('#game').boundingBox();
    await page.mouse.dblclick(rect.x+(city.x+city.w/2)*rect.width/f.width,rect.y+(city.y+city.h*.6)*rect.height/768);
    await idle();
    await page.waitForFunction(()=>frame.draws.some(d=>d.kind==='text'&&/population/i.test(d.text)),{timeout:15000});
    await click('Choose production');
    for(let n=0;n<80;n++){await key('ArrowDown');
      if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes('Warriors')))break}
    await key('Enter');
    await shot('city-window');
    await key('Escape');
  }catch(e){console.error('city-window failed:',e.message)}
  await key('Control+s');await save('normal');
  await shot('map');

  // Science advisor.
  try{await key('F6');await shot('science');await key('Escape')}
  catch(e){console.error('science failed:',e.message)}

  // Rates dialog (Cancel discards the draft).
  try{await page.keyboard.press('Shift+KeyT');await shot('rates');await key('Escape')}
  catch(e){console.error('rates failed:',e.message)}

  // City status report.
  try{await key('F1');await shot('report-cities');await key('Escape')}
  catch(e){console.error('report-cities failed:',e.message)}

  // Save dialog with the name field.
  try{await key('Control+s');await fill('shot');await shot('save-dialog');await key('Escape');await key('Escape')}
  catch(e){console.error('save-dialog failed:',e.message)}

  // Wide layout with the message column after a few played rounds.
  try{
    await resizeViewport(1792,768);
    for(let n=0;n<3;n++){
      await research();
      const turn=+((await state()).draws.map(d=>d.text?.match(/(?:^| · )Turn (\d+)(?:$| · )/)).find(Boolean)?.[1]??0);
      await key('Control+n');
      if(has(await state(),'End your turn with movable units?'))await key('Enter');
      await page.waitForFunction(t=>frame.phase==='Human'&&frame.draws.some(d=>+(d.text?.match(/(?:^| · )Turn (\d+)(?:$| · )/)?.[1])===t+1),turn,{timeout:60000});
      await idle();
    }
    await shot('map-wide');
  }catch(e){console.error('map-wide failed:',e.message)}

  // Every image reference in the guide must resolve from docs/, where
  // manual.md lives. The first push linked screenshots/… instead of
  // manual/screenshots/… and every image on GitHub 404'd.
  const manual=path.join(repo,'docs','manual.md');
  const refs=[...fs.readFileSync(manual,'utf8').matchAll(/\]\(([^)]+\.png)\)/g)].map(m=>m[1]);
  const broken=refs.filter(ref=>!path.isAbsolute(ref)&&!fs.existsSync(path.resolve(repo,'docs',ref)));
  if(broken.length)throw Error('guide image references do not resolve: '+broken.join(', '));
  console.log('verified',refs.length,'guide image references resolve');

  await browser.close();
})().catch(e=>{console.error('FATAL',e);process.exitCode=1})
  .finally(()=>{server.kill('SIGKILL');setTimeout(()=>process.exit(process.exitCode||0),500)});
