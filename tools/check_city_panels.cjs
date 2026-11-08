// Recorded panel routes use actual canvas/keyboard input, never game-state injection.
const fs=require('fs'),path=require('path');
exports.run=async ({page,state,key,click,save,load,idle,fixture,saves,out})=>{
 for(const route of JSON.parse(fs.readFileSync(path.join(fixture,'route.json'),'utf8'))){
  const name='panel'+route.name;
  fs.copyFileSync(path.join(fixture,route.name+'-before.cth'),path.join(saves,name+'.cth'));
  await load(name);
  if((await state()).draws.some(d=>d.text==='Manage city'))await click('Manage city');
  await click('Info');
  for(const step of route.inputs){
   const input=step.input;
   if(input.type==='key')await key(input.key.replace('Ctrl+','Control+'));
   else if(input.type==='click'){await page.mouse.click(input.x,input.y);await idle()}
   else throw Error('unsupported panel input');
   const current=await state();
   if(!current.draws.some(d=>d.kind==='text'&&d.x===744&&d.y===193&&d.text===step.panel))throw Error('panel transition differs '+route.name);
   if(step.text&&!current.draws.some(d=>d.text===step.text))throw Error('unit roster count differs '+route.name);
   if(step.modal&&!current.draws.some(d=>d.text===step.modal))throw Error('sale confirmation missing '+route.name);
   if(!step.modal&&current.draws.some(d=>d.text==='Confirm improvement sale'))throw Error('sale confirmation did not close');
   if(!step.modal&&step.pages>1&&!current.draws.some(d=>d.text===`List ${step.page} / ${step.pages}`))throw Error('list pagination differs '+route.name);
  }
  const f=await state();
  if(!f.draws.some(d=>d.kind==='text'&&d.x===744&&d.y===193&&d.text===route.panel))throw Error('wrong displayed panel '+route.name);
  if(!f.draws.some(d=>d.kind==='text'&&d.x===18&&d.y===14&&d.text===route.heading))throw Error('wrong displayed city '+route.name);
  await page.screenshot({path:path.join(out,name+'.png')});
  await key('Control+s');await save(name+'after');
  if(!fs.readFileSync(path.join(saves,name+'after.cth')).equals(fs.readFileSync(path.join(fixture,route.name+'-after.cth'))))throw Error('panel checkpoint differs '+route.name);
  await load(name+'after');
  if(!(await state()).draws.some(d=>d.kind==='text'&&d.x===744&&d.y===193&&d.text===route.panel))throw Error('panel was reset on reload '+route.name);
  await key('Control+s');await save(name+'reload');
  if(!fs.readFileSync(path.join(saves,name+'reload.cth')).equals(fs.readFileSync(path.join(fixture,route.name+'-after.cth'))))throw Error('panel reload checkpoint differs '+route.name);
 }
 await key('Escape');
 // #178: a different session loaded from the map defaults to Info, including
 // when its saved production budget immediately reopens the city workspace.
 await load('panelbudgettabs');
 if(!(await state()).draws.some(d=>d.kind==='text'&&d.x===744&&d.y===193&&d.text==='Info'))throw Error('map load retained old city panel');
 await key('Control+s');await save('panelnewsession');
 if(!fs.readFileSync(path.join(saves,'panelnewsession.cth')).equals(fs.readFileSync(path.join(fixture,'budgettabs-before.cth'))))throw Error('Info default changed loaded gameplay');
 await click('Happy');
 await load('panelnewsession');
 if(!(await state()).draws.some(d=>d.kind==='text'&&d.x===744&&d.y===193&&d.text==='Happy'))throw Error('open workspace lost its panel on load');
 await key('Control+s');await save('panelnewsessionreload');
 if(!fs.readFileSync(path.join(saves,'panelnewsessionreload.cth')).equals(fs.readFileSync(path.join(fixture,'budgettabs-before.cth'))))throw Error('open workspace load changed gameplay');
 await key('Escape');
};
