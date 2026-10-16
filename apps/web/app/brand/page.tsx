import type { Metadata } from "next";
import "./brand.css";

export const metadata: Metadata = {
  title: "Orcel Brand Identity",
  description: "The Orcel identity: one white disc, two diagonal rails, a trusted path.",
};

const basePath = process.env.ORCEL_WEB_BASE_PATH ?? "";
const asset = (file: string) => basePath + "/brand/" + file;
const kit = "https://github.com/TheHalfMoon/Orcel/raw/refs/heads/main/assets/brand/downloads/Orcel-Identity-Kit-v1.zip";
const palette = [
  {name:"White",hex:"#FFFFFF"},{name:"Ink",hex:"#080808"},
  {name:"Charcoal",hex:"#17191B"},{name:"Silver",hex:"#D9DDE1"},
  {name:"Mist",hex:"#F5F6F7"},{name:"Graphite",hex:"#565D66"}
];

export default function BrandPage() {
 return (
  <main className="orcel-brand-system">
    <div className="brand-shell">
      <header className="brand-nav">
        <a href={basePath+"/"} aria-label="Return to Orcel home">← orcel</a>
        <span>BRAND SYSTEM / V1.0</span>
        <a href={kit}>Download kit ↗</a>
      </header>
      <section className="brand-intro">
        <div className="brand-eyebrow">THE ORCEL IDENTITY</div>
        <h1>One mark.<br/>Two rails.</h1>
        <p>A confident white disc. Two transparent diagonal cuts.
        A recognizable symbol of user intent and trusted, local control.</p>
      </section>
      <section className="brand-hero" aria-label="Orcel primary mark on black">
        <img src={asset("orcel-o-white.svg")} width="256" height="256"
          alt="Orcel logo: white disc with two parallel rounded diagonal cutouts"/>
        <div className="brand-hero-caption">
          <span>01 / PRIMARY SYMBOL</span><strong>ORCEL</strong>
          <span>White / Ink</span>
        </div>
      </section>
      <section className="brand-heading">
        <h2>Built for every surface.</h2>
        <p>One geometric source; a considered system of variants.</p>
      </section>
      <section className="brand-tiles">
        <article className="brand-tile dark">
          <p>02 / DARK APPLICATION</p>
          <img src={asset("orcel-app-icon.svg")} width="210" height="210" alt="Orcel dark app tile"/>
          <span>ChatGPT · Codex · App</span>
        </article>
        <article className="brand-tile light">
          <p>03 / LIGHT APPLICATION</p>
          <img src={asset("orcel-app-icon-light.svg")} width="210" height="210" alt="Orcel light app tile"/>
          <span>Documentation · Light UI</span>
        </article>
        <article className="brand-tile dark">
          <p>04 / WORDMARK</p>
          <img className="brand-wide" src={asset("orcel-lockup-white.svg")} width="425" height="112" alt="Orcel symbol and custom vector wordmark"/>
          <span>Horizontal master lockup</span>
        </article>
        <article className="brand-tile light">
          <p>05 / INVERSE</p>
          <img className="brand-wide" src={asset("orcel-lockup-black.svg")} width="425" height="112" alt="Black Orcel lockup on white"/>
          <span>Horizontal inverse lockup</span>
        </article>
      </section>
      <section className="brand-palette" aria-labelledby="color-title">
        <div className="brand-heading"><h2 id="color-title">The palette.</h2><p>Nothing decorative in the logo. Neutral materials around it.</p></div>
        <div className="brand-swatches">
          {palette.map(p=><div className="brand-swatch" key={p.hex}>
           <span className="brand-swatch-color" style={{backgroundColor:p.hex}}/>
           <strong>{p.name}</strong><code>{p.hex}</code>
          </div>)}
        </div>
      </section>
      <section className="brand-principle">
        <span>INTEGRITY BY DESIGN</span>
        <h2>The mark remains unchanged.<br/>The system scales around it.</h2>
        <p>Keep its circular silhouette, two rounded diagonal cuts, and clearspace.
        No gradients, mascot, shadows, or additional marks inside the symbol.
        The original historical logo and evidence are preserved separately.</p>
        <a className="brand-download" href={kit}>Download complete Orcel identity kit ↗</a>
      </section>
      <footer className="brand-bottom">
        <a href={basePath+"/"}>Orcel</a>
        <span>Your computer, your terms.</span>
        <a href="https://github.com/TheHalfMoon/Orcel/tree/main/docs/brand">Identity documentation ↗</a>
      </footer>
    </div>
  </main>
 );
}
