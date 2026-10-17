import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import test from "node:test";
const root=join(dirname(fileURLToPath(import.meta.url)),"..","..","..");
const file=(path)=>readFileSync(join(root,path));
const str=(path)=>file(path).toString("utf8");
const hash=(path)=>createHash("sha256").update(file(path)).digest("hex");

test("Approved nested Orcel monogram keeps O r c e l geometry in sync",()=>{
 const svg=str("assets/brand/orcel-o-white.svg");
 assert.match(svg,/viewBox="0 0 256 256"/);
 assert.match(svg,/<rect x="18" y="20" width="188" height="216" rx="70"/);
 for(const path of ["M154 63 H105","M169 92 H124","M177 120 H145","M122 145 H175","M234 79 V201"])
   assert.ok(svg.includes(path),path);
 assert.equal((svg.match(/<path /g)||[]).length,5);
 assert.equal((svg.match(/<circle /g)||[]).length,0);
 assert.doesNotMatch(svg,/satellite|orbit|<text|<image|linearGradient/i);
 assert.equal(str("apps/web/public/brand/orcel-o-white.svg"),svg);
 assert.match(str("assets/brand/orcel-o-black.svg"),/stroke="#080808"/);
});

test("OpenAI/Codex app icons and Next favicon are byte-identical",()=>{
 const master=file("assets/brand/orcel-app-icon.svg");
 for(const path of [
  "apps/web/public/brand/orcel-app-icon.svg",
  "apps/web/app/icon.svg",
  "distribution/openai/assets/logo.svg",
  "distribution/codex/orcel/assets/logo.svg"
 ]) assert.deepEqual(file(path),master,path);
 const openai=JSON.parse(str("distribution/openai/plugin.config.json"));
 const codex=JSON.parse(str("distribution/codex/orcel/plugin.json"));
 assert.equal(openai.brandColor,"#080808");
 assert.equal(codex.extensions["com.openai"].interface.brandColor,"#080808");
});

test("Identity delivers full mono and editorial system without distributing fonts",()=>{
 const names=[
 "orcel-o-white.svg","orcel-o-black.svg",
 "orcel-app-icon.svg","orcel-app-icon-light.svg","orcel-accent-icon.svg",
 "orcel-wordmark-white.svg","orcel-wordmark-black.svg",
 "orcel-lockup-white.svg","orcel-lockup-black.svg",
 "orcel-lockup-stacked-white.svg","orcel-lockup-stacked-black.svg",
 "orcel-social-dark.svg","orcel-social-light.svg","orcel-monogram-pattern.svg"
 ];
 for (const name of names) {
  assert.ok(statSync(join(root,"assets/brand",name)).size>175,name);
 }
 for (const name of ["orcel-wordmark-white.svg","orcel-wordmark-black.svg","orcel-lockup-white.svg"]) {
  const svg=str("assets/brand/"+name);
  assert.doesNotMatch(svg,/<text\b|<image\b|@font-face/i,"no font or image dependencies");
 }
 assert.match(str("assets/brand/orcel-wordmark-white.svg"),/stroke-width="19"/);
 const page=str("apps/web/app/brand/page.tsx");
 assert.match(page,/Nested by design/);
 assert.match(page,/nested strokes/);
 assert.match(page,/orcel-lockup-white\.svg/);
 assert.match(page,/Download kit/);
 assert.match(str("apps/web/app/page.tsx"),/\/brand\.html/);
 assert.ok(statSync(join(root,"apps/web/app/brand/brand.css")).size>2000);
});

test("Every raster export is validated and delivered via deterministic ZIP",()=>{
 const manifest=JSON.parse(str("assets/brand/ASSET_MANIFEST.json"));
 assert.equal(manifest.name,"Orcel");
 assert.equal(manifest.brand_version,"2.0");
 assert.ok(manifest.assets.length>=50);
 for (const item of manifest.assets) {
  assert.equal(hash(item.path),item.sha256,item.path);
  assert.equal(file(item.path).byteLength,item.bytes,item.path);
 }
 for (const n of [16,32,64,256,1024]) {
  const png=file("assets/brand/exports/orcel-app-dark-"+n+".png");
  assert.equal(png.subarray(0,8).toString("hex"),"89504e470d0a1a0a");
  assert.equal(png.readUInt32BE(16),n);
  assert.equal(png.readUInt32BE(20),n);
 }
 const zip=file("assets/brand/downloads/Orcel-Identity-Kit-v2.zip");
 assert.equal(zip.subarray(0,4).toString("hex"),"504b0304");
 assert.ok(zip.length>25000&&zip.length<3000000);
});

test("Historic brand release artifact is preserved without change",()=>{
 assert.equal(hash("apps/web/public/brand/orcel-mark-mask.svg"),
  "56bd8af1c84680e45ff579a130e16949a472e81c6846e475adf6f06f16e92daf");
});
