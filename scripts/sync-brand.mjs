#!/usr/bin/env node
// Orcel Orbital Identity. One deterministic vector construction, shared across
// site, OpenAI/Codex packages, favicon, marketing, and downloadable assets.
// Based on the founder-approved October 16, 2026 orbital concept. Not third-party art.
// No font files, runtime dependencies, or mutable external resources.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo=join(dirname(fileURLToPath(import.meta.url)),"..");
const svgStart='<svg xmlns="http://www.w3.org/2000/svg"';
function doc(view,body,label) {
 return svgStart+' viewBox="'+view+'" role="img" aria-label="'+label+'">\n'+body+'\n</svg>\n';
}
// Master geometry (256x256). Outer C-shaped orbit deliberately ends before
// the detached satellite and near the lower vertical axis. Inner O is complete.
export const ORBIT={
  outerPath:"M188 36 A108 108 0 1 0 128 232",
  outerWidth:10,
  inner:{cx:137,cy:127,r:78,width:12},
  satellite:{cx:208,cy:49,r:14}
};
function core(ink) {
 const {outerPath,outerWidth,inner,satellite}=ORBIT;
 return '<path d="'+outerPath+'" stroke="'+ink+'" stroke-width="'+outerWidth+'" fill="none" stroke-linecap="round"/>'+
  '<circle cx="'+inner.cx+'" cy="'+inner.cy+'" r="'+inner.r+'" stroke="'+ink+'" stroke-width="'+inner.width+'" fill="none"/>'+
  '<circle cx="'+satellite.cx+'" cy="'+satellite.cy+'" r="'+satellite.r+'" fill="'+ink+'"/>';
}
const pure=doc("0 0 256 256","  "+core("#FFFFFF"),"Orcel orbital emblem: open outer white arc, inner white ring, satellite dot");
const inverse=doc("0 0 256 256","  "+core("#080808"),"Orcel orbital emblem in black for a light surface");
const tile=doc("0 0 256 256",'  <rect width="256" height="256" rx="54" fill="#080808"/>\n  '+core("#FFFFFF"),"Orcel dark application icon with orbital symbol");
const tileLight=doc("0 0 256 256",'  <rect width="256" height="256" rx="54" fill="#F5F6F7"/>\n  '+core("#080808"),"Orcel light application icon with orbital symbol");
// Custom hand-authored lowercase outline letterforms, converted to filled
// outlines by a 19px monoline stroke. No system or redistributed font needed.
function lettering(ink) {
 return '<g fill="none" stroke="'+ink+'" stroke-width="19" stroke-linejoin="round" stroke-linecap="round">'+
  '<circle cx="46" cy="81" r="27.5"/>'+
  '<path d="M103 109V80 Q103 55 126 55"/>'+
  '<path d="M198 60 C174 44 151 59 151 82 C151 105 175 119 198 103"/>'+
  '<path d="M231 83 H287 C287 64 275 54 259 54 C241 54 225 67 225 85 C225 105 241 115 258 115 C270 115 280 111 287 104"/>'+
  '<path d="M327 27 V109"/>'+
  '</g>';
}
const wordWhite=doc("0 0 357 142","  "+lettering("#FFFFFF"),"Lowercase Orcel white vector wordmark");
const wordBlack=doc("0 0 357 142","  "+lettering("#080808"),"Lowercase Orcel black vector wordmark");
function horizontal(ink) {
 return doc("0 0 900 256",
   '  <g>'+core(ink)+'</g>\n'+
   '  <g transform="translate(270 23) scale(1.65)">'+lettering(ink)+'</g>',
   "Orcel orbital emblem and lowercase custom wordmark in horizontal arrangement");
}
const lockWhite=horizontal("#FFFFFF"),lockBlack=horizontal("#080808");
function stacked(ink) {
 return doc("0 0 520 450",
  '  <g transform="translate(132 15)">'+core(ink)+'</g>\n'+
  '  <g transform="translate(80 278) scale(1.04)">'+lettering(ink)+'</g>',
  "Orcel vertical lockup with orbital icon and lowercase wordmark");
}
const stackWhite=stacked("#FFFFFF"),stackBlack=stacked("#080808");
function social(dark) {
 const ink=dark?"#FFFFFF":"#080808",bg=dark?"#080808":"#F5F6F7",muted=dark?"#B8BCC1":"#565D66";
 return doc("0 0 1200 630",
   '<rect width="1200" height="630" fill="'+bg+'"/>\n'+
   '<g transform="translate(90 151) scale(1.28)">'+core(ink)+'</g>\n'+
   '<g transform="translate(420 200) scale(1.77)">'+lettering(ink)+'</g>\n'+
   '<path d="M95 529 H1105" stroke="'+muted+'" stroke-width="1" opacity=".4"/>\n'+
   '<text x="420" y="453" font-family="Arial,sans-serif" font-size="30" fill="'+muted+'">Your computer, your terms.</text>\n'+
   '<text x="98" y="578" font-family="Arial,sans-serif" font-size="18" letter-spacing="2.5" fill="'+muted+'">OPEN SOURCE  /  LOCAL-FIRST  /  USER-APPROVED</text>',
   "Orcel open-source project social banner");
}
function motif() {
 const parts=['<rect width="1200" height="420" fill="#080808"/>'];
 for (let i=0;i<5;i++) {
  const x=30+i*230;
  parts.push('<g transform="translate('+x+' 70) scale(.9)" opacity="'+(0.12+i*.11).toFixed(2)+'">'+core("#FFFFFF")+'</g>');
 }
 return doc("0 0 1200 420",parts.join("\n"),"Orcel orbital rhythm for supporting brand graphics");
}
const pattern=motif();
// A tightly limited accent treatment for optional marketing pieces only.
// The primary identity and package tiles remain strictly monochrome.
const accent=doc("0 0 256 256",
  '  <rect width="256" height="256" rx="54" fill="#17191B"/>\n'+
  '  <path d="'+ORBIT.outerPath+'" stroke="#D9DDE1" stroke-width="'+ORBIT.outerWidth+'" fill="none" stroke-linecap="round"/>\n'+
  '  <circle cx="137" cy="127" r="78" stroke="#FFFFFF" stroke-width="12" fill="none"/>\n'+
  '  <circle cx="208" cy="49" r="14" fill="#FFFFFF"/>',
  "Orcel graphite/silver accent icon for editorial use");
const outputs=new Map([
 ["assets/brand/orcel-o-white.svg",pure],
 ["assets/brand/orcel-o-black.svg",inverse],
 ["assets/brand/orcel-app-icon.svg",tile],
 ["assets/brand/orcel-app-icon-light.svg",tileLight],
 ["assets/brand/orcel-wordmark-white.svg",wordWhite],
 ["assets/brand/orcel-wordmark-black.svg",wordBlack],
 ["assets/brand/orcel-lockup-white.svg",lockWhite],
 ["assets/brand/orcel-lockup-black.svg",lockBlack],
 ["assets/brand/orcel-lockup-stacked-white.svg",stackWhite],
 ["assets/brand/orcel-lockup-stacked-black.svg",stackBlack],
 ["assets/brand/orcel-social-dark.svg",social(true)],
 ["assets/brand/orcel-social-light.svg",social(false)],
 ["assets/brand/orcel-orbit-pattern.svg",pattern],
 ["assets/brand/orcel-accent-icon.svg",accent],
 ["apps/web/public/brand/orcel-o-white.svg",pure],
 ["apps/web/public/brand/orcel-o-black.svg",inverse],
 ["apps/web/public/brand/orcel-app-icon.svg",tile],
 ["apps/web/public/brand/orcel-app-icon-light.svg",tileLight],
 ["apps/web/public/brand/orcel-lockup-white.svg",lockWhite],
 ["apps/web/public/brand/orcel-lockup-black.svg",lockBlack],
 ["apps/web/public/brand/orcel-lockup-stacked-white.svg",stackWhite],
 ["apps/web/public/brand/orcel-lockup-stacked-black.svg",stackBlack],
 ["apps/web/public/brand/orcel-social-dark.svg",social(true)],
 ["apps/web/public/brand/orcel-social-light.svg",social(false)],
 ["apps/web/public/brand/orcel-orbit-pattern.svg",pattern],
 ["apps/web/public/brand/orcel-accent-icon.svg",accent],
 ["apps/web/app/icon.svg",tile],
 ["distribution/openai/assets/logo.svg",tile],
 ["distribution/codex/orcel/assets/logo.svg",tile]
]);
const check=process.argv.includes("--check"),errors=[];
for (const [name,contents] of outputs) {
 const path=join(repo,...name.split("/"));
 if(check) {
  let actual;
  try { actual=readFileSync(path,"utf8"); } catch { actual=undefined; }
  if(actual!==contents)errors.push(name);
 } else {
  mkdirSync(dirname(path),{recursive:true});writeFileSync(path,contents,"utf8");
 }
}
if(errors.length) {console.error("Brand asset drift:\n"+errors.join("\n"));process.exit(1);}
console.log("Orcel orbital identity "+(check?"verified":"generated")+": "+outputs.size+" SVG destinations");
for (const [name,data] of [["symbol",pure],["app tile",tile],["lockup",lockWhite]]) {
 console.log(name+": "+createHash("sha256").update(data).digest("hex"));
}
