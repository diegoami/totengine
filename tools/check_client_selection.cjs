// Real pointer/keyboard selection regression. The fixture is generated through
// normal play; metadata is used only to locate visible units and assert results.
const fs=require('fs'),path=require('path');
exports.run=async({page,state,key,click,save,load,idle,fixture,saves,out})=>{
 const meta=JSON.parse(fs.readFileSync(path.join(fixture,'route.json')));
 for(const name of ['before','stack'])fs.copyFileSync(path.join(fixture,name+'.cth'),path.join(saves,'selection'+name+'.cth'));
 const a=meta.source.slot,b=meta.target.slot;
 const unit=async slot=>{const u=(await state()).draws.find(d=>d.kind==='unit_sprite'&&d.unit===slot);if(!u)throw Error('visible unit absent '+slot);return u};
 async function expect(slot,moving=true){const u=await unit(slot);if(!u.selected)throw Error('wrong selected identity, expected '+slot);const f=await state();if(!f.draws.some(d=>d.text===(moving?'Moving [V]':'Viewing [V]')))throw Error('wrong input mode');return u}
 async function pointer(slot,part=1){const u=await unit(slot),p=meta.hit_points[part];const viewport=page.viewportSize(),scale=Math.min(viewport.width/1024,viewport.height/768);await page.mouse.click((viewport.width-1024*scale)/2+(u.x+p.x*u.w)*scale,(viewport.height-768*scale)/2+(u.y+p.y*u.h)*scale);await idle()}
 async function compare(name,wanted){await key('Control+s');await save(name);if(!fs.readFileSync(path.join(saves,name+'.cth')).equals(fs.readFileSync(path.join(fixture,wanted+'.cth'))))throw Error('selection checkpoint differs '+name)}
 await load('selectionbefore');await expect(a);
 await pointer(b,0);await expect(b); // DIFFERENT unit while already Moving.
 for(const slot of [a,b])if((await unit(slot)).moves_left!==3)throw Error('selection spent movement');
 const revision=(await state()).revision,counterBefore=await page.evaluate(()=>counter);
 const p=meta.hit_points[1],u=await unit(b),other=await unit(a);
 const sample=async()=>page.evaluate(({u,other,p})=>{
  const rgb=d=>{const c=ctx.getImageData(Math.floor(d.x+p.x*d.w),Math.floor(d.y+p.y*d.h),1,1).data;return '#'+[...c].slice(0,3).map(x=>x.toString(16).padStart(2,'0')).join('')};return [rgb(u),rgb(other)];
 },{u,other,p});
 const pixels=new Set(),others=new Set();
 for(let i=0;i<12;i++){const [active,stable]=await sample();pixels.add(active);others.add(stable);await page.waitForTimeout(100)}
 if(pixels.size<2||!pixels.has(p.rgb)||others.size!==1)throw Error('selected sprite must blink while the other unit stays visible');
 if((await state()).revision!==revision||await page.evaluate(()=>counter)!==counterBefore)throw Error('blink generated input');
 // Hidden sprite must still receive pointer input. Do not change the clock.
 await page.waitForFunction(()=>Math.floor(performance.now()/500)%2===1&&performance.now()%500<180);
 await pointer(b,1);await expect(b);
 if(!(await state()).status.startsWith("Unit activated."))throw Error("hidden-phase click missed the unit");
 await pointer(a,2);await expect(a);await key('v');await expect(a,false);
 await pointer(b,2);await expect(b);
 await compare('selectioninert','before');
 await page.setViewportSize({width:1360,height:900});await key('z');
 await pointer(a,1);await expect(a);await pointer(b,1);await expect(b);
 await key('Shift+z');await page.setViewportSize({width:1024,height:768});
 await compare('selectionresized','before');
 // Stack chooser selects a stable member and has no gameplay side effect.
 await load('selectionstack');const stackUnit=(await state()).draws.find(d=>d.kind==='unit_sprite'&&d.own);
 await pointer(stackUnit.unit,1);
 if(!(await state()).draws.some(d=>d.text==='Activate a unit'))throw Error('stack chooser absent');
 const choice=(await state()).draws.find(d=>d.text?.includes('#'+b+' ·'));
 if(!choice)throw Error('stack member absent');await click(choice.text);await key('Enter');await expect(b);
 await compare('selectionstackinert','stack');
 await load('selectionbefore');await pointer(b,1);await expect(b);
 let actor=await unit(b);const dx=meta.move_delta[0],dy=meta.move_delta[1];
 await page.mouse.click(actor.x+actor.w/2+dx*32,actor.y+actor.h*.75+dy*16);await idle();
 await expect(a);if((await unit(b)).moves_left!==0)throw Error('moved unit not exhausted');
 await compare('selectionmove','after-move');
 await pointer(b,1);await expect(b,false);if((await unit(b)).blink)throw Error('exhausted unit is blinking');
 await key('v');await expect(a);await key('Space');
 if(!(await state()).draws.some(d=>d.text==='End of turn'))throw Error('end-of-turn state absent');
 await compare('selectionskip','after-skip');
 await key('Enter');await page.waitForFunction(turn=>frame.phase==='Human'&&frame.draws.some(d=>d.text?.startsWith('Turn '+turn+' ')),meta.turn+1);
 await idle();
 if((await state()).draws.some(d=>d.text==='Choose research explicitly')){await key('ArrowDown');await key('Enter')}
 if(!(await state()).draws.some(d=>d.kind==='unit_sprite'&&d.selected&&d.blink&&d.moves_left>0))throw Error('new human turn did not select a ready unit');
 await compare('selectionturn','after-turn');
 await page.screenshot({path:path.join(out,'selection.png')});
 console.log('Different-unit selection, stack, hidden-phase hit, sprite blink, zoom/resize, exhaustion and next turn: pass');
};
