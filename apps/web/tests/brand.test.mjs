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

test("Design 17 is one disc with two transparent rounded diagonal rails",()=>{
 const svg=str("assets/brand/orcel-o-white.svg");
 assert.match(svg,/viewBox="0 0 256 256"/);
 assert.match(svg,/fill="#FFFFFF"/);
 assert.match(svg,/fill-rule="evenodd"/);
 assert.match(svg,/M128 20 A108 108/);
 assert.equal((svg.match(/A10 10 0 0 1/g)??[]).length,4);
 assert.doesNotMatch(svg,/filter=|linearGradient|radialGradient|#1F4E79|<text\b|<image\b/i);
 assert.equal(str("apps/web/public/brand/orcel-o-white.svg"),svg);
 assert.equal(str("assets/brand/orcel-o-black.svg").includes('fill="#080808"'),true);
});

test("Both LLM plugin packages use the exact same current Orcel tile",()=>{
 const master=file("assets/brand/orcel-app-icon.svg");
 for(const p of [
  "distribution/openai/assets/logo.svg",
  "distribution/codex/orcel/assets/logo.svg",
  "apps/web/public/brand/orcel-app-icon.svg",
  "apps/web/app/icon.svg"
 ])assert.deepEqual(file(p),master,p);
 const openai=JSON.parse(str("distribution/openai/plugin.config.json"));
 const codex=JSON.parse(str("distribution/codex/orcel/plugin.json"));
 assert.equal(openai.brandColor,"#080808");
 assert.equal(codex.extensions["com.openai"].interface.brandColor,"#080808");
});

test("Complete identity includes true-vector lockups, invertible marks, and accessible public gallery",()=>{
 for(const name of [
  "orcel-o-black.svg","orcel-app-icon-light.svg","orcel-lockup-white.svg",
  "orcel-lockup-black.svg","orcel-wordmark-white.svg","orcel-wordmark-black.svg",
  "orcel-social-dark.svg","orcel-social-light.svg","orcel-rails-pattern.svg"
 ])assert.ok(statSync(join(root,"assets/brand",name)).size>150,name);
 for(const p of ["assets/brand/orcel-wordmark-white.svg","assets/brand/orcel-wordmark-black.svg"])
  assert.doesNotMatch(str(p),/<text\b/i,"wordmark uses bespoke vectors without external fonts");
 const page=str("apps/web/app/brand/page.tsx");
 assert.match(page,/One mark/);
 assert.match(page,/two transparent diagonal cuts/i);
 assert.match(page,/orcel-app-icon-light.svg/);
 assert.match(page,/brand kit|Download kit/i);
 const index=str("apps/web/app/page.tsx");
 assert.match(index,/brandUrl/);
 assert.match(index,/\/brand\.html/);
 assert.ok(statSync(join(root,"apps/web/app/brand/brand.css")).size>2000);
});

test("Raster deliverables, sizes, provenance, and deterministic ZIP are packaged",()=>{
 const manifest=JSON.parse(str("assets/brand/ASSET_MANIFEST.json"));
 assert.equal(manifest.name,"Orcel");
 assert.equal(manifest.brand_version,"1.0");
 assert.ok(manifest.assets.length>=45);
 for(const item of manifest.assets){
   assert.equal(hash(item.path),item.sha256,item.path);
   assert.equal(file(item.path).byteLength,item.bytes,item.path);
 }
 for(const n of [16,32,64,256,1024]){
   const bytes=file("assets/brand/exports/orcel-app-dark-"+n+".png");
   assert.equal(bytes.subarray(0,8).toString("hex"),"89504e470d0a1a0a");
   assert.equal(bytes.readUInt32BE(16),n);
   assert.equal(bytes.readUInt32BE(20),n);
 }
 const zip=file("assets/brand/downloads/Orcel-Identity-Kit-v1.zip");
 assert.equal(zip.subarray(0,4).toString("hex"),"504b0304");
 assert.ok(zip.length>25000&&zip.length<2e6);
});

test("Historic brand record has not been repurposed to the current mark",()=>{
 assert.equal(
  hash("apps/web/public/brand/orcel-mark-mask.svg"),
  "56bd8af1c84680e45ff579a130e16949a472e81c6846e475adf6f06f16e92daf"
 );
});
