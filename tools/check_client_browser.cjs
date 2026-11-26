const {chromium}=require('playwright');
const fs=require('fs');
const demo=process.env.TOTENGINE_OPEN_DEMO==='1';
const startLabel=demo?'Start open demo':'Start game (Original rules)';
const unitChoice=demo?'Guards':'Warriors';
const origin=process.env.TOTENGINE_CLIENT_ORIGIN||'http://127.0.0.1:8773';
const out=process.env.TOTENGINE_CLIENT_REPORT||'/tmp/totengine-client';
fs.mkdirSync(out,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox'],executablePath:process.env.TOTENGINE_BROWSER_EXECUTABLE});
 const page=await browser.newPage({viewport:{width:1024,height:768}});page.on('pageerror',e=>console.error('PAGEERROR',e));
 await page.goto(origin);
 async function idle(){await page.waitForFunction(()=>typeof frame!=='undefined'&&frame&&!busy&&paintedRevision===frame.revision)}
 async function state(){await idle();return page.evaluate(()=>frame)}
 async function resizeViewport(width,height){
  const previous=page.viewportSize();
  if(previous.width===width&&previous.height===height){await idle();return}
  const display=page.waitForResponse(response=>response.url().endsWith('/input')&&response.request().postDataJSON()?.input?.type==='display');
  await page.setViewportSize({width,height});
  await display;
  const wanted=Math.min(1792,Math.max(1024,Math.floor(width*768/height)));
  // Resize reports are asynchronous. Await the acknowledged logical stage
  // before sending a key; an in-flight display update deliberately drops it.
  await page.waitForFunction(w=>frame?.width===w&&!busy&&!pendingDisplay&&paintedRevision===frame.revision,wanted);
 }
 async function key(k){await idle();await page.keyboard.press(k);await idle()}
 async function click(label){
  const f=await state(),t=f.draws.find(d=>d.kind==='text'&&d.text===label);
  if(!t)throw Error('Missing '+label+' '+f.status);
  const rect=await page.locator('#game').boundingBox();
  await page.mouse.click(rect.x+(t.x+18)*rect.width/f.width,rect.y+(t.y+9)*rect.height/768);
  await idle();
 }
 async function fill(value){await idle();await page.locator('#entry').fill(value);await page.waitForFunction(v=>frame.entry?.value===v&&!busy,value);await idle()}
 function has(f,t){return f.draws.some(d=>d.kind==='text'&&d.text===t)}
 function turnOf(f){return +f.draws.map(d=>d.text?.match(/(?:^| · )Turn (\d+)(?:$| · )/)).find(Boolean)[1]}
 async function research(){if(has(await state(),'Choose research')){await key('ArrowDown');await key('Enter');await page.waitForFunction(()=>frame.status==='research-set')}}
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
 await click(startLabel);
 if((await state()).phase!==setupPhase||(await state()).status!=='Window is unfocused; input ignored')throw Error('delayed focus defeated newer blur');
 await page.unroute('**/*',holdFocusRequests);
 await restoreFocus();
 const knowledge=process.env.TOTENGINE_KNOWLEDGE_PREFLIGHT;
 if(knowledge){
  await click('Open scenario (experimental)');
  for(let n=0;n<40&&!has(await state(),knowledge);n++)await key('PageDown');
  await click(knowledge);
  const f=await state(),t=f.draws.filter(d=>d.kind==='text').map(d=>d.text).join(' ').replace(/\s+/g,' ');
  if(!t.includes('starts from saved map knowledge')||!t.includes('later play uses engine visibility/contact rules'))throw Error('saved-knowledge policy missing from preflight');
  if(!t.includes('original ending and scoring screens'))throw Error('preflight lost ending disclosure');
  await page.screenshot({path:out+'/knowledge-preflight.png'});
  await key('Escape');await key('Escape');
  if((await state()).phase!==setupPhase)throw Error('cancelled preflight changed setup');
 }
 await idle();await click('Generated seed');await fill('7');await click(startLabel);
 await click('Save pending choice');await save('research');await research();
 if(process.env.TOTENGINE_CLASSIC_UNITS==='1'){
  const saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  const unit=(f,prefix)=>f.draws.find(d=>d.sprite?.startsWith(prefix));
  if(!unit(await state(),'unit-')||unit(await state(),'classic-unit-'))throw Error('classic unit art is on by default');
  await key('Control+s');await save('unitbefore');
  await click('Game');await click('Settings…');await click('[ ] Classic unit art');await key('Escape');
  const map=await state(),art=unit(map,'classic-unit-');
  if(!art)throw Error('classic unit art missing from map');
  const sprite=await page.request.get(origin+'/sprite/'+map.assets+'/'+art.sprite);
  if(!sprite.ok()||(await sprite.body()).length!==8+64*64*4)throw Error('classic unit sprite was not served');
  await key('Control+s');await save('unitclassic');
  const read=name=>fs.readFileSync(require('path').join(saves,name+'.cth'));
  if(!read('unitbefore').equals(read('unitclassic')))throw Error('classic unit art affected the checkpoint');
  await click('Game');await click('Settings…');await click('[x] Classic unit art');await key('Escape');
  if(!unit(await state(),'unit-')||unit(await state(),'classic-unit-'))throw Error('classic unit art did not turn off');
 }
 await key('F3');
 if(!has(await state(),'Known contacts')||!has(await state(),'No known contacts.'))throw Error('F3 contact panel missing');
 await key('Escape');
 await key('b');await click('Save pending choice');await save('founding');await fill('athenx');await page.keyboard.press('Backspace');await page.keyboard.type('a');await page.waitForFunction(()=>frame.entry?.value==='athena'&&!busy);await key('Enter');
 await click('Choose production');
 for(let n=0;n<80;n++){await key('ArrowDown');if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes(unitChoice)))break}
 await key('Enter');await key('Control+s');await save('normal');
 if(process.env.TOTENGINE_CLASSIC_ICONS==='1'){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  const glyph=(f,name)=>f.draws.some(d=>d.kind==='sprite'&&d.sprite==='classic-'+name);
  if(glyph(await state(),'food'))throw Error('classic icons are on by default');
  await click('Game');await click('Settings…');
  await click('[ ] Classic resource icons');await key('Escape');
  const hud=await state();
  if(!glyph(hud,'food'))throw Error('classic food glyph missing from map');
  const sprite=await page.request.get(origin+'/sprite/'+hud.assets+'/classic-food');
  if(!sprite.ok()||(await sprite.body()).length!==8+14*14*4)throw Error('classic sprite was not served');
  await click('Manage city');
  const city=await state();
  if(!['food','shields','science','tax','luxury'].every(name=>glyph(city,name)))throw Error('classic glyph missing from city');
  await key('Escape');await key('Control+s');await save('classicicons');
  const read=name=>fs.readFileSync(path.join(saves,name+'.cth'));
  if(!read('classicicons').equals(read('normal')))throw Error('changing icons affected the checkpoint');
  await click('Game');await click('Settings…');await click('[x] Classic resource icons');await key('Escape');
  if(glyph(await state(),'food'))throw Error('turning icons off left a glyph on the map');
 }
 await page.screenshot({path:out+'/browser-map.png'});
 for(let n=0;n<10;n++){
  await research();const turn=turnOf(await state());
  await key('Control+n');if(has(await state(),'End your turn with movable units?'))await key('Enter');
  await page.waitForFunction(turn=>frame.phase==='Human'&&frame.draws.some(d=>+(d.text?.match(/(?:^| · )Turn (\d+)(?:$| · )/)?.[1])===turn+1),turn,{timeout:30000});await idle();
 }
 await research();await key('Control+s');await save('tenrounds');
 await resizeViewport(1440,1000);await page.screenshot({path:out+'/browser-round11-1440.png'});
 const final=await state();console.log(JSON.stringify({phase:final.phase,status:final.status,turn:turnOf(final)}));
 // Resume through real dialog input; checkpoints must preserve pending choices.
 await resizeViewport(1024,768);
 async function load(name){await key('Control+l');await fill(name);await key('Enter')}
 await load('research');await research();await key('b');await click('Save pending choice');await save('researchnext');
 await loadPending('founding');await fill('athena');await key('Enter');
 await click('Choose production');
 for(let n=0;n<80;n++){await key('ArrowDown');if((await state()).draws.some(d=>d.text?.startsWith('●')&&d.text.includes(unitChoice)))break}
 await key('Enter');await key('Control+s');await save('foundingnext');
 await load('normal');await key('Control+n');if(has(await state(),'End your turn with movable units?'))await key('Enter');
 await page.waitForFunction(()=>frame.phase==='Human'&&frame.draws.some(d=>/(?:^| · )Turn 2(?:$| · )/.test(d.text)));await idle();
 await research();await key('Control+s');await save('normalnext');
 async function loadPending(name){if(has(await state(),'Name your city'))await key('Escape');await load(name)}
 const peace=process.env.TOTENGINE_PEACE_FIXTURES;
 if(peace){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  for(const [name,result] of [['accept','accepted'],['refuse','refused']]){
   const slot='peace'+name,pending=slot+'pending';
   const read=name=>fs.readFileSync(path.join(saves,name+'.cth'));
   fs.copyFileSync(path.join(peace,'peace-'+name+'.cth'),path.join(saves,slot+'.cth'));
   await load(slot);await research();await key('F3');await click('Talk to civ 2');
   await click('Review peace proposal');await key('Escape');await click('Review peace proposal');
   await click('Save pending choice');await save(pending);
   for(const run of ['direct','resumed']){
    if(run==='resumed'){await click('Load checkpoint');await save(pending)}
    await click('Send peace proposal');
    if(!(await state()).draws.some(d=>d.text?.startsWith('The proposal was '+result+'.')))throw Error('peace result differs '+slot);
    await click('Save pending choice');await save(slot+run);
   }
   if(!read(slot+'direct').equals(read(slot+'resumed')))throw Error('peace replay differs '+slot);
   await click('Return to contacts');await key('Escape');
  }
  await load('normal');
 }
 const victory=process.env.TOTENGINE_VICTORY_FIXTURES;
 if(victory){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  const initial=path.join(victory,'victoryinitial.cth');
  if(fs.existsSync(initial)){
   fs.copyFileSync(initial,path.join(saves,'victoryinitial.cth'));
   await load('victoryinitial');await idle();
   if(!has(await state(),'Choose research'))throw Error('saved initial view lost required research');
   await click('Save pending choice');await save('savedviewinitial');
   if(!fs.readFileSync(path.join(saves,'savedviewinitial.cth')).equals(fs.readFileSync(initial)))throw Error('saved initial view checkpoint differs');
   await page.screenshot({path:out+'/saved-view-initial.png'});
   await research();await key('Control+s');await save('savedviewplayed');
   if(!fs.readFileSync(path.join(saves,'savedviewplayed.cth')).equals(fs.readFileSync(path.join(victory,'victorystart.cth'))))throw Error('saved-view engine continuation differs');
  }
  fs.copyFileSync(path.join(victory,'victorystart.cth'),path.join(saves,'victorystart.cth'));
  await load('victorystart');await key('Control+n');
  if(has(await state(),'End your turn with movable units?'))await key('Enter');
  await page.waitForFunction(()=>frame.phase==='GameOver');await idle();
  for(const expected of ['Video not played:','Controlled victory.','Game over: scenario ENDGAME.']){
   if(!(await state()).draws.some(d=>d.text?.includes(expected)))throw Error('missing ending box '+expected);
   await key('Enter');
  }
  await page.screenshot({path:out+'/victory.png'});
  await key('Control+n');await key('Control+s');await save('victoryafter');
  const expected=fs.readFileSync(path.join(victory,'victoryend.cth'));
  if(!fs.readFileSync(path.join(saves,'victoryafter.cth')).equals(expected))throw Error('terminal checkpoint differs');
  await load('victoryafter');
  if((await state()).phase!=='GameOver')throw Error('terminal state lost on load');
  for(let i=0;i<3;i++)await key('Enter');
  await key('Control+s');await save('victoryrestored');
  if(!fs.readFileSync(path.join(saves,'victoryrestored.cth')).equals(expected))throw Error('restored ending checkpoint differs');
  await load('normal');
 }
 const production=process.env.TOTENGINE_PRODUCTION_FIXTURES;
 if(production){
  const path=require('path'),saves=process.env.TOTENGINE_CLIENT_SAVES;
  if(!saves)throw Error('set TOTENGINE_CLIENT_SAVES');
  for(const route of JSON.parse(fs.readFileSync(path.join(production,'route.json'),'utf8'))){
   const name='production'+route.name;
   fs.copyFileSync(path.join(production,route.name+'-before.cth'),path.join(saves,name+'.cth'));
   await load(name);
   for(const input of route.inputs){
    if(input.type==='key')await key(input.key.replace('Ctrl+','Control+'));
    else if(input.type==='click'){await page.mouse.click(input.x,input.y);await idle()}
    else throw Error('unsupported production route input');
   }
   await page.screenshot({path:out+'/'+name+'.png'});
   await key('Control+s');await save(name+'after');
   if(!fs.readFileSync(path.join(saves,name+'after.cth')).equals(fs.readFileSync(path.join(production,route.name+'-after.cth'))))throw Error('production checkpoint differs '+route.name);
   // Replay the persisted result before following another route.
   await load(name+'after');
  }
 }
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
 if(process.env.TOTENGINE_GOVERNMENT_FIXTURES)await require('./check_government_route.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_GOVERNMENT_FIXTURES,saves:process.env.TOTENGINE_CLIENT_SAVES,out});
 if(process.env.TOTENGINE_REPORTS_FIXTURES)await require('./check_government_route.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_REPORTS_FIXTURES,saves:process.env.TOTENGINE_CLIENT_SAVES,out,start:'repstart',shot:'reports-route-final.png'});
 if(process.env.TOTENGINE_PEDIA_FIXTURES){
  const compact=process.env.TOTENGINE_PEDIA_COMPACT==='1';
  if(compact)await resizeViewport(960,720);
  await require('./check_government_route.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_PEDIA_FIXTURES,saves:process.env.TOTENGINE_CLIENT_SAVES,out,start:'pedstart',startTitle:'Choose research',shot:'pedia-route-final.png'});
  if(compact)await resizeViewport(1024,768);
 }
 if(process.env.TOTENGINE_PANELS_FIXTURES)await require('./check_city_panels.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_PANELS_FIXTURES,saves:process.env.TOTENGINE_CLIENT_SAVES,out});
 if(process.env.TOTENGINE_CITY_OPEN_FIXTURE)await require('./check_city_open.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_CITY_OPEN_FIXTURE,saves:process.env.TOTENGINE_CLIENT_SAVES,out});
 if(process.env.TOTENGINE_CITY_OPEN_FIXTURE)await require('./check_city_gesture_races.cjs').run({page,state,key,click,save,load,idle,fixture:process.env.TOTENGINE_CITY_OPEN_FIXTURE,saves:process.env.TOTENGINE_CLIENT_SAVES,out});
 // Human-phase reconnect preserves gameplay. The gesture-race suite above
 // separately checks a second tab continuing an authorized opponent round.
 const beforeReconnect=await (await page.request.get(origin+'/state')).json();
 const tab=await browser.newPage({viewport:{width:1024,height:768}});await tab.goto(origin);await tab.waitForFunction(()=>typeof frame!=='undefined'&&frame);
 const afterReconnect=await tab.evaluate(()=>frame);
 if(beforeReconnect.phase==='Human'&&(afterReconnect.phase!=='Human'||JSON.stringify(afterReconnect.draws)!==JSON.stringify(beforeReconnect.draws)))throw Error('human-phase reconnect changed the pending view');
 await tab.close();
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});
