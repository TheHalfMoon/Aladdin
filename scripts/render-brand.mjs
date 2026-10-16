#!/usr/bin/env node
// Build-time only. Uses the already exact-pinned sharp dependency of apps/web.
// No bundled font files, online API, or graphics runtime in Orcel's product.
import { createRequire } from "node:module";
import { mkdirSync,readFileSync,copyFileSync } from "node:fs";
import { resolve,dirname,join } from "node:path";
import { fileURLToPath } from "node:url";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const require=createRequire(join(root,"apps/web/package.json"));
const sharp=require("sharp");
const out=join(root,"assets/brand/exports");
mkdirSync(out,{recursive:true});
const batch=[];
const add=(src,w,h,name)=>batch.push({src,w,h,name});
for(const n of [16,24,32,48,64,128,256,512,1024]){
 add("orcel-app-icon.svg",n,n,"orcel-app-dark-"+n+".png");
 add("orcel-o-white.svg",n,n,"orcel-mark-white-"+n+".png");
}
for(const n of [32,64,128,256,512,1024]){
 add("orcel-app-icon-light.svg",n,n,"orcel-app-light-"+n+".png");
 add("orcel-o-black.svg",n,n,"orcel-mark-black-"+n+".png");
}
for(const color of ["white","black"]){
 add("orcel-wordmark-"+color+".svg",714,284,"orcel-wordmark-"+color+".png");
 add("orcel-lockup-"+color+".svg",1350,345,"orcel-lockup-"+color+".png");
}
for(const kind of ["dark","light"])add("orcel-social-"+kind+".svg",1200,630,"orcel-social-"+kind+".png");
const fails=[];
for(const {src,w,h,name} of batch){
 try {
   const path=join(root,"assets/brand",src);
   const raw=readFileSync(path);
   const vb=raw.toString("utf8").match(/viewBox="0 0 ([0-9]+) ([0-9]+)"/);
   const w0=Number(vb?.[1]||256),h0=Number(vb?.[2]||256);
   const density=Math.min(800,Math.max(72,Math.ceil(72*Math.max(w/w0,h/h0))));
   await sharp(raw,{density}).resize(w,h,{fit:"fill"}).png({compressionLevel:9}).toFile(join(out,name));
 }catch(e){fails.push(name+": "+e.message)}
}
if(fails.length){console.error(fails.join("\n"));process.exit(1)}
copyFileSync(join(out,"orcel-social-dark.png"),join(root,"apps/web/public/brand/orcel-social-dark.png"));
console.log("Rendered "+batch.length+" PNG files from authored Orcel SVG variants using local sharp.");
