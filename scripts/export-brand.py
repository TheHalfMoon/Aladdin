#!/usr/bin/env python3
"""Orcel brand kit raster exports and deterministic downloadable package.
Build-time helper only: first run node scripts/render-brand.mjs (local sharp), then Pillow.
Source of truth: scripts/sync-brand.mjs.
"""
from __future__ import annotations
from hashlib import sha256
from io import BytesIO
from pathlib import Path
from zipfile import ZipFile, ZipInfo, ZIP_DEFLATED
import json
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "assets" / "brand"
OUT = BRAND / "exports"
ZIPPATH = BRAND / "downloads" / "Orcel-Identity-Kit-v1.zip"
OUT.mkdir(parents=True, exist_ok=True)
ZIPPATH.parent.mkdir(parents=True, exist_ok=True)

def png(svg_name: str, size: tuple[int,int], dest: str) -> Path:
    result = OUT / dest
    if not result.is_file():
        raise FileNotFoundError(f"Missing {dest}; first run node scripts/render-brand.mjs")
    with Image.open(result) as im:
        if im.size != size:
            raise ValueError(f"Wrong dimensions for {dest}: {im.size} != {size}")
    return result

def font(size: int):
    for src in [r"C:\Windows\Fonts\segoeuib.ttf",
                r"C:\Windows\Fonts\segoeui.ttf"]:
        if Path(src).is_file():
            return ImageFont.truetype(src, size=size)
    return ImageFont.load_default()

def thumb(path: Path, dims: tuple[int,int]) -> Image.Image:
    im = Image.open(path).convert("RGBA")
    im.thumbnail(dims, Image.Resampling.LANCZOS)
    return im

for size in (16,24,32,48,64,128,256,512,1024):
    png("orcel-app-icon.svg", (size,size), f"orcel-app-dark-{size}.png")
    png("orcel-o-white.svg", (size,size), f"orcel-mark-white-{size}.png")
for size in (32,64,128,256,512,1024):
    png("orcel-app-icon-light.svg", (size,size), f"orcel-app-light-{size}.png")
    png("orcel-o-black.svg", (size,size), f"orcel-mark-black-{size}.png")
png("orcel-wordmark-white.svg",(714,284),"orcel-wordmark-white.png")
png("orcel-wordmark-black.svg",(714,284),"orcel-wordmark-black.png")
png("orcel-lockup-white.svg",(1350,345),"orcel-lockup-white.png")
png("orcel-lockup-black.svg",(1350,345),"orcel-lockup-black.png")
png("orcel-social-dark.svg",(1200,630),"orcel-social-dark.png")
png("orcel-social-light.svg",(1200,630),"orcel-social-light.png")

# Icon smoke test: solid white disc, transparent holes, transparent background.
mark = Image.open(OUT / "orcel-mark-white-256.png").convert("RGBA")
assert mark.getpixel((128,60))[3] > 250, "disc face must be opaque"
assert mark.getpixel((108,121))[3] < 6, "first rail must be cut out"
assert mark.getpixel((150,151))[3] < 6, "second rail must be cut out"
assert mark.getpixel((0,0))[3] == 0, "primary mark must be transparent outside disc"

# Portable brand presentation preview.
W,H=1600,1060
canvas=Image.new("RGB",(W,H),(9,10,11))
draw=ImageDraw.Draw(canvas)
title,body,small=font(47),font(24),font(17)
white=(250,250,250); silver=(172,178,185); ink=(13,14,15)
draw.text((72,53),"ORCEL",font=title,fill=white)
draw.text((74,114),"IDENTITY SYSTEM  /  DESIGN 17  /  V1.0",font=small,fill=silver)
draw.line((74,163,1524,163), fill=(61,63,65),width=2)
def panel(x,y,w,h,bg,border=None):
    draw.rounded_rectangle((x,y,x+w,y+h),radius=26,fill=bg,outline=border,width=2 if border else 1)
def paste(file,x,y,width,height):
    im=thumb(OUT/file,(width,height))
    canvas.paste(im,(x,y),im)
panel(74,202,710,573,(22,24,26),(48,51,54))
panel(804,202,720,573,(244,245,246))
paste("orcel-app-dark-512.png",210,245,440,440)
paste("orcel-app-light-512.png",950,245,440,440)
draw.text((107,717),"PRIMARY / DARK",font=body,fill=white)
draw.text((841,717),"INVERSE / LIGHT",font=body,fill=(28,30,32))
draw.text((81,814),"THE MARK",font=body,fill=white)
draw.text((81,852),"One white disc. Two parallel diagonal rails.",font=font(25),fill=silver)
draw.text((81,890),"Intent and local approval. One trusted path.",font=font(22),fill=silver)
draw.text((820,815),"ICON SIZES",font=body,fill=white)
x=820
for n in (128,64,32,16):
    fi=f"orcel-app-dark-{n}.png"
    im=Image.open(OUT/fi).convert("RGBA")
    canvas.paste(im,(x,872),im)
    draw.text((x,1008),f"{n}px",font=small,fill=silver)
    x+=n+65
canvas.save(OUT/"orcel-identity-overview.png",optimize=True)

# .ico for downstream desktop/window clients
ico = Image.open(OUT/"orcel-app-dark-256.png").convert("RGBA")
ico.save(OUT/"orcel-favicon.ico",sizes=[(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)])

sources=sorted(BRAND.glob("*.svg"))
exports=sorted(OUT.glob("*"))
guide=[BRAND/"IDENTITY.md",ROOT/"docs"/"brand"/"README.md"]
entries=[*sources,*exports,*[p for p in guide if p.exists()]]
manifest={"name":"Orcel","brand_version":"1.0","source":"Design 17","assets":[]}
for path in entries:
    manifest["assets"].append({
        "path":path.relative_to(ROOT).as_posix(),
        "sha256":sha256(path.read_bytes()).hexdigest(),
        "bytes":path.stat().st_size
    })
manifest_bytes=(json.dumps(manifest,indent=2,ensure_ascii=False)+"\n").encode("utf-8")
(BRAND/"ASSET_MANIFEST.json").write_bytes(manifest_bytes)
entries.append(BRAND/"ASSET_MANIFEST.json")
with ZipFile(ZIPPATH,"w",ZIP_DEFLATED,compresslevel=9) as z:
    for path in sorted(entries,key=lambda p:p.as_posix()):
        relative=path.relative_to(ROOT).as_posix()
        info=ZipInfo("Orcel-Identity-Kit-v1/"+relative,date_time=(2026,10,16,0,0,0))
        info.compress_type=ZIP_DEFLATED
        info.external_attr=0o644<<16
        z.writestr(info,path.read_bytes(),compress_type=ZIP_DEFLATED,compresslevel=9)
print("PNG/Raster exports:",len(exports))
print("Manifest entries:",len(manifest["assets"]))
print("Bundle:",ZIPPATH.name,"bytes",ZIPPATH.stat().st_size)
print("Preview:",(OUT/"orcel-identity-overview.png").relative_to(ROOT))
