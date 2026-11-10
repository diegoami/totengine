// Recorded science/rates/government route through real browser input (#196).
const fs=require('fs'),path=require('path');
const keys={'Ctrl+n':'Control+n','Ctrl+s':'Control+s','Shift+t':'Shift+T','Shift+r':'Shift+R'};
function title(f){const d=f.draws.find(d=>d.kind==='text'&&d.x===212&&d.y===125&&d.size===22);return d?d.text:null}
exports.run=async ({page,state,key,click,save,load,idle,fixture,saves,out,start='govstart',shot='government-route-final.png'})=>{
 const route=JSON.parse(fs.readFileSync(path.join(fixture,'route.json'),'utf8'));
 fs.copyFileSync(path.join(fixture,start+'.cth'),path.join(saves,start+'.cth'));
 await load(start);
 if(title(await state())!==null)throw Error('start checkpoint did not return to the map');
 for(const [n,step] of route.entries()){
  if(step.op==='key')await key(keys[step.key]||step.key);
  else if(step.op==='button')await click(step.label);
  else if(step.op==='entry')await save(step.value);
  else if(step.op==='research'){const f=await state();if(f.draws.some(d=>d.text==='Choose research explicitly')){await key('ArrowDown');await key('Enter');await page.waitForFunction(()=>frame.status==='research-set')}}
  else if(step.op==='endturn'){
   const f=await state();const turn=parseInt(f.draws.find(d=>d.text?.startsWith('Turn ')).text.split(' ')[1]);
   await key('Control+n');if((await state()).draws.some(d=>d.text==='End your turn with movable units?'))await key('Enter');
   await page.waitForFunction(t=>frame.phase==='Human'&&frame.draws.some(d=>d.text?.startsWith('Turn '+t+' ')),turn+1);await idle();
  }
  else if(step.op==='compare'){
   if(!fs.readFileSync(path.join(saves,step.name+'.cth')).equals(fs.readFileSync(path.join(fixture,step.name+'.cth'))))throw Error('checkpoint differs '+step.name);
   continue;
  }
  else throw Error('unsupported government step '+step.op);
  const shown=title(await state());
  if(shown!==(step.title??null))throw Error(`step ${n} ${JSON.stringify(step)}: dialog is ${shown}`);
 }
 await page.screenshot({path:path.join(out,shot)});
};
