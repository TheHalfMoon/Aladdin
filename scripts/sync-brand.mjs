#!/usr/bin/env node
// Orcel brand system. One canonical, reproducible vector geometry.
// Founder selected design 17: solid circular silhouette, two diagonal rounded
// negative-space rails. No third-party brand art, fonts, or runtime dependencies.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
const svgStart = '<svg xmlns="http://www.w3.org/2000/svg"';
const circle = 'M128 20 A108 108 0 1 0 128 236 A108 108 0 1 0 128 20 Z';
const f = (n) => Number(n.toFixed(3));
function capsule(x1, y1, x2, y2, r) {
  const dx=x2-x1,dy=y2-y1,len=Math.hypot(dx,dy),nx=-dy/len,ny=dx/len;
  const a=[f(x1+nx*r),f(y1+ny*r)],b=[f(x2+nx*r),f(y2+ny*r)];
  const c=[f(x2-nx*r),f(y2-ny*r)],e=[f(x1-nx*r),f(y1-ny*r)];
  return 'M'+a.join(' ')+' L'+b.join(' ')+' A'+r+' '+r+' 0 0 1 '+c.join(' ')+
    ' L'+e.join(' ')+' A'+r+' '+r+' 0 0 1 '+a.join(' ')+' Z';
}
export const MARK_PATH = [circle, capsule(83,146,133,96,10), capsule(125,177,176,126,10)].join(' ');
function core(ink) { return '<path d="'+MARK_PATH+'" fill="'+ink+'" fill-rule="evenodd"/>'; }
function doc(view,body,description) {
  return svgStart+' viewBox="'+view+'" role="img" aria-label="'+description+'">\n'+body+'\n</svg>\n';
}
const pure=doc('0 0 256 256','  '+core('#FFFFFF'),'Orcel white circular symbol with two rounded diagonal cuts');
const inverse=doc('0 0 256 256','  '+core('#080808'),'Orcel black circular symbol with two rounded diagonal cuts');
const tile=doc('0 0 256 256','  <rect width="256" height="256" rx="54" fill="#080808"/>\n  '+core('#FFFFFF'),'Orcel dark app icon');
const tileLight=doc('0 0 256 256','  <rect width="256" height="256" rx="54" fill="#F5F6F7"/>\n  '+core('#080808'),'Orcel light app icon');
// Custom vector wordmark: five individually drawn round-geometric lowercase glyphs.
// Unlike SVG <text>, its contours do not depend on installed fonts.
function lettering(ink) {
  return '<g fill="none" stroke="'+ink+'" stroke-width="13.5" stroke-linejoin="round" stroke-linecap="round">'+
   '<circle cx="46" cy="81" r="27.5"/>'+
   '<path d="M103 109V80 Q103 55 126 55"/>'+
   '<path d="M198 60 C174 44 151 59 151 82 C151 105 175 119 198 103"/>'+
   '<path d="M231 83 H287 C287 64 275 54 259 54 C241 54 225 67 225 85 C225 105 241 115 258 115 C270 115 280 111 287 104"/>'+
   '<path d="M327 27 V109"/>'+
   '</g>';
}
const wordWhite=doc('0 0 357 142','  '+lettering('#FFFFFF'),'Orcel custom vector wordmark white');
const wordBlack=doc('0 0 357 142','  '+lettering('#080808'),'Orcel custom vector wordmark black');
function lockup(ink) {
 return doc('0 0 900 230',
  '  <g transform="translate(8 -13) scale(1)">'+core(ink)+'</g>\n'+
  '  <g transform="translate(275 19) scale(1.60)">'+lettering(ink)+'</g>',
  'Orcel horizontal symbol and wordmark');
}
const lockWhite=lockup('#FFFFFF'), lockBlack=lockup('#080808');
function social(dark) {
 const fg=dark?'#FFFFFF':'#080808',bg=dark?'#080808':'#F5F6F7',muted=dark?'#B8BCC1':'#565D66';
 return doc('0 0 1200 630',
  '  <rect width="1200" height="630" fill="'+bg+'"/>\n'+
  '  <g transform="translate(70 171) scale(1.05)">'+core(fg)+'</g>\n'+
  '  <g transform="translate(362 196) scale(1.65)">'+lettering(fg)+'</g>\n'+
  '  <text x="380" y="455" font-family="Arial,Helvetica,sans-serif" font-size="30" letter-spacing="1.2" fill="'+muted+'">Your computer, your terms.</text>\n'+
  '  <path d="M72 543 H1128" stroke="'+muted+'" stroke-width="1" opacity=".4"/>\n'+
  '  <text x="75" y="579" font-family="Arial,Helvetica,sans-serif" font-size="17" letter-spacing="3" fill="'+muted+'">LOCAL-FIRST  /  TRUSTED CONTROL  /  MCP</text>',
  'Orcel social media preview');
}
function motif() {
 const rail1=capsule(83,146,133,96,10),rail2=capsule(125,177,176,126,10);
 let parts=['<rect width="1200" height="420" fill="#080808"/>'];
 for(let j=0;j<6;j++) {
   let x=30+j*210;
   parts.push('<g transform="translate('+x+' 70) scale(1.13)" opacity="'+(0.16+j*0.09).toFixed(2)+'" fill="#FFFFFF" fill-rule="evenodd"><path d="'+rail1+' '+rail2+'"/></g>');
 }
 return doc('0 0 1200 420',parts.join('\n'),'Orcel twin-rail supporting pattern');
}
const outputs=new Map([
 ['assets/brand/orcel-o-white.svg',pure],
 ['assets/brand/orcel-o-black.svg',inverse],
 ['assets/brand/orcel-app-icon.svg',tile],
 ['assets/brand/orcel-app-icon-light.svg',tileLight],
 ['assets/brand/orcel-wordmark-white.svg',wordWhite],
 ['assets/brand/orcel-wordmark-black.svg',wordBlack],
 ['assets/brand/orcel-lockup-white.svg',lockWhite],
 ['assets/brand/orcel-lockup-black.svg',lockBlack],
 ['assets/brand/orcel-social-dark.svg',social(true)],
 ['assets/brand/orcel-social-light.svg',social(false)],
 ['assets/brand/orcel-rails-pattern.svg',motif()],
 ['apps/web/public/brand/orcel-o-white.svg',pure],
 ['apps/web/public/brand/orcel-o-black.svg',inverse],
 ['apps/web/public/brand/orcel-app-icon.svg',tile],
 ['apps/web/public/brand/orcel-app-icon-light.svg',tileLight],
 ['apps/web/public/brand/orcel-lockup-white.svg',lockWhite],
 ['apps/web/public/brand/orcel-lockup-black.svg',lockBlack],
 ['apps/web/public/brand/orcel-social-dark.svg',social(true)],
 ['apps/web/public/brand/orcel-social-light.svg',social(false)],
 ['apps/web/app/icon.svg',tile],
 ['distribution/openai/assets/logo.svg',tile],
 ['distribution/codex/orcel/assets/logo.svg',tile]
]);
const check=process.argv.includes('--check'), errors=[];
for(const [name,contents] of outputs){
 const path=join(repo,...name.split('/'));
 if(check){
   let actual;try{actual=readFileSync(path,'utf8')}catch{actual=undefined}
   if(actual!==contents)errors.push(name);
 }else{
   mkdirSync(dirname(path),{recursive:true});writeFileSync(path,contents,'utf8');
 }
}
if(errors.length){console.error('Orcel brand drift:\n'+errors.join('\n'));process.exit(1);}
console.log('Orcel brand '+(check?'verified':'generated')+': '+outputs.size+' synchronized SVG assets.');
for(const [name,content] of [['white',pure],['dark-app',tile],['lockup',lockWhite]])console.log(name+': '+createHash('sha256').update(content).digest('hex'));
