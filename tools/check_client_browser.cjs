const {chromium}=require('playwright');
const fs=require('fs');
const origin=process.env.TOTENGINE_CLIENT_ORIGIN||'http://127.0.0.1:8773';
const out=process.env.TOTENGINE_CLIENT_REPORT||'/tmp/totengine-client';
fs.mkdirSync(out,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
 const page=await browser.newPage({viewport:{width:1024,height:768}});page.on('pageerror',e=>console.error('PAGEERROR',e));
 await page.goto(origin);
 async function idle(){await page.waitForFunction(()=>typeof frame!=='undefined'&&frame&&!busy&&paintedRevision===frame.revision)}
 async function state(){await idle();return page.evaluate(()=>frame)}
 async function key(k){await idle();await page.keyboard.press(k);await idle()}
 async function click(label){const f=await state();const t=f.draws.find(d=>d.kind==='text'&&d.text===label);if(!t)throw Error('Missing '+label+' '+f.status);await page.mouse.click(t.x+18,t.y+9);await idle()}
 async function fill(value){await idle();await page.locator('#entry').fill(value);await page.waitForFunction(v=>frame.entry?.value===v&&!busy,value);await idle()}
 function has(f,t){return f.draws.some(d=>d.kind==='text'&&d.text===t)}
 async function research(){if(has(await state(),'Choose research explicitly')){await key('ArrowDown');await key('Enter');await page.waitForFunction(()=>frame.status==='research-set')}}
 async function save(name){await fill(name);await key('Enter')}
 await idle();await click('Start Original surface game');
 await click('Save pending choice');await save('research');await research();
 await key('f');await click('Save pending choice');await save('founding');await fill('athenx');await page.keyboard.press('Backspace');await page.keyboard.type('a');await page.waitForFunction(()=>frame.entry?.value==='athena'&&!busy);await key('Enter');
 await click('Choose production');
 for(let n=0;n<80;n++){await key('ArrowDown');if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes('Warriors')))break}
 await key('Enter');await key('s');await save('normal');
 await page.screenshot({path:out+'/browser-map.png'});
 for(let n=0;n<10;n++){
  await research();const f=await state();const turn=+f.draws.find(d=>d.text?.match(/^Turn (\d+) /)).text.match(/^Turn (\d+)/)[1];
  await key('e');if(has(await state(),'End your turn with movable units?'))await key('Enter');
  await page.waitForFunction(turn=>frame.phase==='Human'&&frame.draws.some(d=>d.text?.startsWith('Turn '+(turn+1)+' ')),turn,{timeout:30000});await idle();
 }
 await research();await key('s');await save('tenrounds');
 await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:out+'/browser-round11-1440.png'});
 const final=await state();console.log(JSON.stringify({phase:final.phase,status:final.status,turn:final.draws.find(d=>d.text?.startsWith('Turn ')).text}));
 // Resume through real dialog input; checkpoints must preserve pending choices.
 await page.setViewportSize({width:1024,height:768});
 async function load(name){await key('l');await fill(name);await key('Enter')}
 await load('research');await research();await key('f');await click('Save pending choice');await save('researchnext');
 await loadPending('founding');await fill('athena');await key('Enter');
 await click('Choose production');
 for(let n=0;n<80;n++){await key('ArrowDown');if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes('Warriors')))break}
 await key('Enter');await key('s');await save('foundingnext');
 await load('normal');await key('e');if(has(await state(),'End your turn with movable units?'))await key('Enter');
 await page.waitForFunction(()=>frame.phase==='Human'&&frame.draws.some(d=>d.text?.startsWith('Turn 2 ')));await idle();
 await research();await key('s');await save('normalnext');
 async function loadPending(name){if(has(await state(),'Name your city'))await key('Escape');await load(name)}
 const citizens=process.env.TOTENGINE_CITIZENS_FIXTURES;
 if(citizens){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  fs.copyFileSync(path.join(citizens,'mature.cth'),path.join(saves,'mature.cth'));
  const meta=JSON.parse(fs.readFileSync(path.join(citizens,'route.json'),'utf8'));
  async function compare(name,expected){await key('s');await save(name);if(!fs.readFileSync(path.join(saves,name+'.cth')).equals(fs.readFileSync(path.join(citizens,expected+'.cth'))))throw Error('citizen checkpoint differs '+name)}
  await load('mature');await key('o');
  if(!(await state()).draws.some(d=>d.text?.includes('population 5')))throw Error('mature city missing');
  const x=500+[1,2,1,0,-1,-2,-1,0,2,2,-2,-2,1,3,3,1,-1,-3,-3,-1,0][meta.worker_tile]*45;
  const y=300+[-1,0,1,2,1,0,-1,-2,-2,2,2,-2,-3,-1,1,3,3,1,-1,-3,0][meta.worker_tile]*25;
  await page.mouse.click(x,y);await idle();await compare('cityentertainer','after-entertainer');
  await click('1: Entertainer');await compare('citytax','after-tax');
  await click('1: Tax collector');await compare('cityscience','after-science');
  await click('1: Scientist');await compare('citycycle','after-cycle');
  await click('Choose production');await key('ArrowDown');await key('Escape');
  await page.screenshot({path:out+'/city-citizens.png'});
  await click('Auto workers (centre tile)');await load('citycycle');
  for(let i=0;i<21;i++){if(has(await state(),'Tile '+meta.worker_tile))break;await key('ArrowRight')}
  await key('Space');await compare('cityresume','after-resume');
  await key('Escape');await key('o');
  if(process.env.TOTENGINE_NAVIGATION_FIXTURE){
   fs.copyFileSync(process.env.TOTENGINE_NAVIGATION_FIXTURE,path.join(saves,'navigation.cth'));await load('navigation');
   const first=(await state()).draws.find(d=>d.text?.includes(' — population ')).text;
   await key('PageDown');if((await state()).draws.find(d=>d.text?.includes(' — population ')).text===first)throw Error('city navigation did not change city');
   await key('PageUp');await key('s');await save('citynavigation');
   if(!fs.readFileSync(path.join(saves,'citynavigation.cth')).equals(fs.readFileSync(process.env.TOTENGINE_NAVIGATION_FIXTURE)))throw Error('navigation mutated gameplay');
  }
  await key('Escape');
 }
 const fixtures=process.env.TOTENGINE_ENCOUNTER_FIXTURES;
 if(fixtures){
  const path=require('path');const saves=process.env.TOTENGINE_CLIENT_SAVES;if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  for(const [name,actor] of [['battle',1],['capture',7]]){
   fs.copyFileSync(path.join(fixtures,'before-'+name+'.cth'),path.join(saves,name+'.cth'));
   await load(name);
   let found=false;for(let i=0;i<80;i++){if((await state()).draws.some(d=>d.text?.endsWith(' #'+actor))){found=true;break}await key('w')}if(!found)throw Error('actor absent');
   if(has(await state(),'Viewing [M]'))await key('m');
   await key('8');await key('Escape');await key('8');await key('Enter');
   await key('s');await save(name+'after');
   if(!fs.readFileSync(path.join(saves,name+'after.cth')).equals(fs.readFileSync(path.join(fixtures,'after-'+name+'.cth'))))throw Error(name+' checkpoint differs');
  }
 }
 // A second tab/reconnect reads the same decision and cannot advance the game.
 const tab=await browser.newPage({viewport:{width:1024,height:768}});await tab.goto(origin);await tab.waitForFunction(()=>typeof frame!=='undefined'&&frame);await tab.close();
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});
