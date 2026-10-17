#!/usr/bin/env node
// Orcel Nested Monogram Identity. One deterministic vector construction, shared across
// site, OpenAI/Codex packages, favicon, marketing, and downloadable assets.
// Based on the founder-approved October 16, 2026 nested monogram design. Not third-party art.
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
// Approved founder-selected nested O/r/c/e/l identity, original SVG geometry.
// One immutable geometry definition shared by website, app and plugin assets.
export const MONOGRAM={
 outer:{x:18,y:20,width:188,height:216,rx:70,widthStroke:10},
 r:"M154 63 H105 C76 63 63 83 63 110 V180 C63 194 77 201 95 201 H156",
 c:"M169 92 H124 C100 92 89 109 89 132 V159 C89 176 103 185 123 185 H171",
 e:"M177 120 H145 C128 120 118 131 118 145 C118 158 128 169 145 169 H177",
 eBar:"M122 145 H175",
 l:"M234 79 V201",
 width:10
};
function core(ink) {
 const m=MONOGRAM;
 const base='fill="none" stroke="'+ink+'" stroke-linecap="round" stroke-linejoin="round"';
 return '<rect x="'+m.outer.x+'" y="'+m.outer.y+'" width="'+m.outer.width+'" height="'+m.outer.height+'" rx="'+m.outer.rx+'" '+base+' stroke-width="'+m.outer.widthStroke+'"/>'+
 [m.r,m.c,m.e,m.eBar,m.l].map(d=>'<path d="'+d+'" '+base+' stroke-width="'+m.width+'"/>').join('');
}
const pure=doc("0 0 256 256","  "+core("#FFFFFF"),"Orcel white nested O r c e l monogram");
const inverse=doc("0 0 256 256","  "+core("#080808"),"Orcel black nested O r c e l monogram");
const tile=doc("0 0 256 256",'  <rect width="256" height="256" rx="54" fill="#080808"/>\n  '+core("#FFFFFF"),"Orcel dark application icon with nested monogram");
const tileLight=doc("0 0 256 256",'  <rect width="256" height="256" rx="54" fill="#F5F6F7"/>\n  '+core("#080808"),"Orcel light application icon with nested monogram");
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
   "Orcel nested monogram and lowercase custom wordmark in horizontal arrangement");
}
const lockWhite=horizontal("#FFFFFF"),lockBlack=horizontal("#080808");
function stacked(ink) {
 return doc("0 0 520 450",
  '  <g transform="translate(132 15)">'+core(ink)+'</g>\n'+
  '  <g transform="translate(80 278) scale(1.04)">'+lettering(ink)+'</g>',
  "Orcel vertical lockup with nested icon and lowercase wordmark");
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
 return doc("0 0 1200 420",parts.join("\n"),"Orcel nested monogram pattern for supporting brand graphics");
}
const pattern=motif();
// A tightly limited accent treatment for optional marketing pieces only.
// The primary identity and package tiles remain strictly monochrome.
const accent=doc("0 0 256 256",
  '  <defs><linearGradient id="accent" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#3B82F6"/><stop offset="1" stop-color="#8B5CF6"/></linearGradient></defs>\n'+
  '  <rect width="256" height="256" rx="54" fill="#10121A"/>\n'+core("url(#accent)"),
  "Orcel optional blue purple editorial accent icon");
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
 ["assets/brand/orcel-monogram-pattern.svg",pattern],
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
 ["apps/web/public/brand/orcel-monogram-pattern.svg",pattern],
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
console.log("Orcel nested monogram identity "+(check?"verified":"generated")+": "+outputs.size+" SVG destinations");
for (const [name,data] of [["symbol",pure],["app tile",tile],["lockup",lockWhite]]) {
 console.log(name+": "+createHash("sha256").update(data).digest("hex"));
}
