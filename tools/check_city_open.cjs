// Original-style city entry: actual double clicks and a deliberately held first POST.
const fs=require('fs'),path=require('path');
exports.run=async ({page,state,key,click,save,load,idle,fixture,saves,out})=>{
 fs.copyFileSync(fixture,path.join(saves,'cityopen.cth'));
 async function map(){await load('cityopen');if((await state()).draws.some(d=>d.text==='Return to map [Esc]'))await key('Escape');await idle()}
 async function point(f){const d=f.draws.find(d=>d.kind==='sprite'&&d.sprite.startsWith('city-'));if(!d)throw Error('city sprite absent');const r=await page.locator('#game').boundingBox();return [r.x+(d.x+d.w/2)*r.width/1024,r.y+(d.y+d.h*.6)*r.height/768]}
 async function opened(){await page.waitForFunction(()=>frame.draws.some(d=>d.kind==='text'&&d.x===18&&d.y===14&&d.text.includes('population')));await idle()}
 async function compare(name){await key('Control+s');await save(name);if(!fs.readFileSync(path.join(saves,name+'.cth')).equals(fs.readFileSync(fixture)))throw Error('double click changed gameplay '+name)}
 await map();let [x,y]=await point(await state());await page.mouse.dblclick(x,y,{delay:50});await opened();await compare('citydouble');
 await page.screenshot({path:path.join(out,'city-double-click.png')});
 await page.setViewportSize({width:1440,height:1000});await map();[x,y]=await point(await state());
 let release,started;const held=new Promise(r=>release=r),ready=new Promise(r=>started=r);
 const hold=async route=>{if(route.request().postDataJSON()?.input?.type==='click'){started();await held;}await route.continue()};
 await page.route('**/input',hold,{times:1});
 await page.mouse.dblclick(x,y,{delay:30});await ready;
 if(!await page.evaluate(()=>pendingDoubleClick!==null))throw Error('busy city double click was dropped');
 release();await opened();await compare('citydoublebusy');await page.unroute('**/input',hold);
 await key('Escape');await page.setViewportSize({width:1024,height:768});
};
