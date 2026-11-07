const {chromium}=require('playwright');
const fs=require('fs');
const origin=process.env.TOTENGINE_CLIENT_ORIGIN||'http://127.0.0.1:8773';
const out=process.env.TOTENGINE_CLIENT_REPORT||'/tmp/totengine-client';
fs.mkdirSync(out,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox'],executablePath:process.env.TOTENGINE_BROWSER_EXECUTABLE});
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
 async function restoreFocus(){
  const posted=page.waitForRequest(request=>request.url().endsWith('/input')&&request.method()==='POST'&&request.postDataJSON()?.input?.type==='focus'&&request.postDataJSON().input.focused===true);
  await page.evaluate(()=>dispatchEvent(new Event('focus')));await posted;await idle();
 }
 // Hold an actual request, then blur while busy. The focus transition must
 // reach the controller after the request instead of being dropped.
 await idle();
 let releaseRequest, requestStarted;
 const started=new Promise(resolve=>{requestStarted=resolve});
 const held=new Promise(resolve=>{releaseRequest=resolve});
 await page.route('**/input',async route=>{requestStarted();await held;await route.continue()},{times:1});
 await page.evaluate(()=>{send({type:'focus',focused:true})});
 await started;
 const blurPosted=page.waitForRequest(request=>request.url().endsWith('/input')&&request.method()==='POST'&&request.postDataJSON()?.input?.type==='focus'&&request.postDataJSON().input.focused===false);
 await page.evaluate(()=>dispatchEvent(new Event('blur')));
 releaseRequest();
 await blurPosted;
 await page.waitForFunction(()=>!busy&&pendingFocus===null);
 await key('Tab');
 if((await state()).status!=='Window is unfocused; input ignored')throw Error('busy blur did not reach controller');
 await restoreFocus();
 // An older focus /state response must not overwrite a later busy blur (#146).
 let releasePost,postStarted,releaseState,stateStarted;
 const postReady=new Promise(resolve=>{postStarted=resolve}),postHeld=new Promise(resolve=>{releasePost=resolve});
 const stateReady=new Promise(resolve=>{stateStarted=resolve}),stateHeld=new Promise(resolve=>{releaseState=resolve});
 let postCaptured=false,stateCaptured=false;
 const holdFocusRequests=async route=>{
  if(route.request().url().endsWith('/input')&&!postCaptured){postCaptured=true;postStarted();await postHeld;await route.continue()}
  else if(route.request().url().endsWith('/state')&&!stateCaptured){stateCaptured=true;const response=await route.fetch();stateStarted();await stateHeld;await route.fulfill({response})}
  else await route.continue();
 };
 await page.route('**/*',holdFocusRequests);
 await page.evaluate(()=>{send({type:'focus',focused:true})});await postReady;
 await page.evaluate(()=>dispatchEvent(new Event('focus')));await stateReady;
 const latestBlurPosted=page.waitForRequest(request=>request.url().endsWith('/input')&&request.method()==='POST'&&request.postDataJSON()?.input?.type==='focus'&&request.postDataJSON().input.focused===false,{timeout:3000});
 await page.evaluate(()=>dispatchEvent(new Event('blur')));
 if(!await page.evaluate(()=>pendingFocus===false))throw Error('later blur was not queued');
 const stateDone=page.waitForResponse(response=>response.url().endsWith('/state'));
 releaseState();await stateDone;
 await page.evaluate(()=>new Promise(resolve=>setTimeout(resolve,0)));
 releasePost();await latestBlurPosted;await page.waitForFunction(()=>!busy&&pendingFocus===null);
 const setupPhase=(await state()).phase;
 await click('Start Original surface game');
 if((await state()).phase!==setupPhase||(await state()).status!=='Window is unfocused; input ignored')throw Error('delayed focus defeated newer blur');
 await page.unroute('**/*',holdFocusRequests);
 await restoreFocus();
 await idle();await click('Start Original surface game');
 await click('Save pending choice');await save('research');await research();
 await key('b');await click('Save pending choice');await save('founding');await fill('athenx');await page.keyboard.press('Backspace');await page.keyboard.type('a');await page.waitForFunction(()=>frame.entry?.value==='athena'&&!busy);await key('Enter');
 await click('Choose production');
 for(let n=0;n<80;n++){await key('ArrowDown');if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes('Warriors')))break}
 await key('Enter');await key('Control+s');await save('normal');
 await page.screenshot({path:out+'/browser-map.png'});
 for(let n=0;n<10;n++){
  await research();const f=await state();const turn=+f.draws.find(d=>d.text?.match(/^Turn (\d+) /)).text.match(/^Turn (\d+)/)[1];
  await key('Control+n');if(has(await state(),'End your turn with movable units?'))await key('Enter');
  await page.waitForFunction(turn=>frame.phase==='Human'&&frame.draws.some(d=>d.text?.startsWith('Turn '+(turn+1)+' ')),turn,{timeout:30000});await idle();
 }
 await research();await key('Control+s');await save('tenrounds');
 await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:out+'/browser-round11-1440.png'});
 const final=await state();console.log(JSON.stringify({phase:final.phase,status:final.status,turn:final.draws.find(d=>d.text?.startsWith('Turn ')).text}));
 // Resume through real dialog input; checkpoints must preserve pending choices.
 await page.setViewportSize({width:1024,height:768});
 async function load(name){await key('Control+l');await fill(name);await key('Enter')}
 await load('research');await research();await key('b');await click('Save pending choice');await save('researchnext');
 await loadPending('founding');await fill('athena');await key('Enter');
 await click('Choose production');
 for(let n=0;n<80;n++){await key('ArrowDown');if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes('Warriors')))break}
 await key('Enter');await key('Control+s');await save('foundingnext');
 await load('normal');await key('Control+n');if(has(await state(),'End your turn with movable units?'))await key('Enter');
 await page.waitForFunction(()=>frame.phase==='Human'&&frame.draws.some(d=>d.text?.startsWith('Turn 2 ')));await idle();
 await research();await key('Control+s');await save('normalnext');
 async function loadPending(name){if(has(await state(),'Name your city'))await key('Escape');await load(name)}
 const citizens=process.env.TOTENGINE_CITIZENS_FIXTURES;
 if(citizens){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  fs.copyFileSync(path.join(citizens,'mature.cth'),path.join(saves,'mature.cth'));
  const meta=JSON.parse(fs.readFileSync(path.join(citizens,'route.json'),'utf8'));
  async function compare(name,expected){await key('Control+s');await save(name);if(!fs.readFileSync(path.join(saves,name+'.cth')).equals(fs.readFileSync(path.join(citizens,expected+'.cth'))))throw Error('citizen checkpoint differs '+name)}
  await load('mature');await click('Manage city');
  if(!has(await state(),'Currently producing:'))throw Error('current production label missing');
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
  await key('Escape');await click('Manage city');
  if(process.env.TOTENGINE_NAVIGATION_FIXTURE){
   fs.copyFileSync(process.env.TOTENGINE_NAVIGATION_FIXTURE,path.join(saves,'navigation.cth'));await load('navigation');
   const first=(await state()).draws.find(d=>d.text?.includes(' — population ')).text;
   await key('PageDown');if((await state()).draws.find(d=>d.text?.includes(' — population ')).text===first)throw Error('city navigation did not change city');
   await key('PageUp');await key('Control+s');await save('citynavigation');
   if(!fs.readFileSync(path.join(saves,'citynavigation.cth')).equals(fs.readFileSync(process.env.TOTENGINE_NAVIGATION_FIXTURE)))throw Error('navigation mutated gameplay');
  }
  await key('Escape');
 }
 if(process.env.TOTENGINE_SELECTION_FIXTURES)await require('./check_client_selection.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_SELECTION_FIXTURES,saves:process.env.TOTENGINE_CLIENT_SAVES,out});
 const movement=process.env.TOTENGINE_MOVEMENT_FIXTURES;
 if(movement){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  fs.copyFileSync(path.join(movement,'before.cth'),path.join(saves,'movement.cth'));
  const route=JSON.parse(fs.readFileSync(path.join(movement,'route.json'),'utf8'));
  async function actor(){for(let i=0;i<100;i++){if((await state()).draws.some(d=>d.text?.endsWith(' #'+route.actor)))return;await key('w')}throw Error('warrior absent')}
  await load('movement');await actor();
  if(has(await state(),'Moving [V]'))await key('v');
  await key('c');
  await page.mouse.click(380,365);await idle();
  await click('Activate unit '+route.actor);
  if(!has(await state(),'Moving [V]'))throw Error('Activate did not enter Moving');
  // Blink must change pixels while leaving the revision and input counter alone.
  const revision=(await state()).revision, counterBefore=await page.evaluate(()=>counter);
  const captures=new Set();
  for(let i=0;i<8;i++){captures.add(await page.evaluate(()=>canvas.toDataURL()));await page.waitForTimeout(120)}
  if(captures.size<2)throw Error('active square did not blink');
  if((await state()).revision!==revision || await page.evaluate(()=>counter)!==counterBefore)throw Error('blink generated input');
  await page.mouse.click(380+route.dx*32,390+route.dy*16);await idle();
  if((await state()).status!=='moved')throw Error('mouse terrain click did not move warrior');
  await key('Control+s');await save('movementafter');
  const expected=fs.readFileSync(path.join(movement,'after.cth'));
  if(!fs.readFileSync(path.join(saves,'movementafter.cth')).equals(expected))throw Error('mouse movement checkpoint differs');
  // Physical keypad code must work even when the browser key label says Home,
  // PageUp, ArrowRight, etc. (Num Lock off).
  const directions={'-1,-1':'7','0,-2':'8','1,-1':'9','-2,0':'4','2,0':'6','-1,1':'1','0,2':'2','1,1':'3'};
  const digit=directions[route.dx+','+route.dy];
  for(const mode of ['on','off','arrows']){
   await load('movement');await actor();
   if(has(await state(),'Viewing [V]'))await key('v');
   if(mode==='on')await key('Numpad'+digit);
   if(mode==='arrows')await key({'4':'ArrowLeft','6':'ArrowRight','8':'ArrowUp','2':'ArrowDown'}[digit]);
   // The off route below uses a real CDP key event with the physical keypad code.
   if(mode==='off'){
    await load('movement');await actor();if(has(await state(),'Viewing [V]'))await key('v');
    const cdp=await page.context().newCDPSession(page);
    const labels={'1':'End','2':'ArrowDown','3':'PageDown','4':'ArrowLeft','6':'ArrowRight','7':'Home','8':'ArrowUp','9':'PageUp'};
    await cdp.send('Input.dispatchKeyEvent',{type:'keyDown',key:labels[digit],code:'Numpad'+digit,location:3,isKeypad:true});await idle();
    await cdp.send('Input.dispatchKeyEvent',{type:'keyUp',key:labels[digit],code:'Numpad'+digit,location:3,isKeypad:true});await cdp.detach();
   }
   await key('Control+s');await save('movementkeypad'+mode);
   if(!fs.readFileSync(path.join(saves,'movementkeypad'+mode+'.cth')).equals(expected))throw Error('keypad movement differs '+mode);
  }
  await page.screenshot({path:out+'/movement.png'});
 }
 const fixtures=process.env.TOTENGINE_ENCOUNTER_FIXTURES;
 if(fixtures){
  const path=require('path');const saves=process.env.TOTENGINE_CLIENT_SAVES;if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  for(const [name,actor] of [['battle',1],['capture',7]]){
   fs.copyFileSync(path.join(fixtures,'before-'+name+'.cth'),path.join(saves,name+'.cth'));
   await load(name);
   let found=false;for(let i=0;i<80;i++){if((await state()).draws.some(d=>d.text?.endsWith(' #'+actor))){found=true;break}await key('w')}if(!found)throw Error('actor absent');
   if(has(await state(),'Viewing [V]'))await key('v');
   await key('8');await key('Escape');await key('8');await key('Enter');
   await key('Control+s');await save(name+'after');
   if(!fs.readFileSync(path.join(saves,name+'after.cth')).equals(fs.readFileSync(path.join(fixtures,'after-'+name+'.cth'))))throw Error(name+' checkpoint differs');
  }
 }
 // A second tab/reconnect reads the same decision and cannot advance the game.
 const tab=await browser.newPage({viewport:{width:1024,height:768}});await tab.goto(origin);await tab.waitForFunction(()=>typeof frame!=='undefined'&&frame);await tab.close();
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});
