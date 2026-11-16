// Renderiza plates (cenários sem elenco) de um projeto Remotion existente: um still por plano.
// O projeto precisa aceitar uma inputProp que esconde o elenco (ex.: `nocast`) no stage/world.
// uso: node plates.mjs --out public/img/plates --composition <Id> --frames 12:s01,80:s02,... [--prop nocast] [--scale 1.3333] [--entry src/index.ts]
import { bundle } from "@remotion/bundler";
import { renderStill, selectComposition } from "@remotion/renderer";
import path from "node:path";
import fs from "node:fs";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const outDir = opt("out", "public/img/plates");
const id = opt("composition");
const list = opt("frames");
if (!id || !list) { console.error("uso: node plates.mjs --composition <Id> --frames f:nome,... [--out dir] [--prop nocast] [--scale 1.3333]"); process.exit(2); }
const inputProps = { [opt("prop", "nocast")]: true };
const scale = Number(opt("scale", "1"));
const browserExecutable = process.env.REMOTION_BROWSER || undefined;
const chromiumOptions = { gl: "angle" }; // three.js precisa de WebGL de verdade no headless
const onBrowserLog = (l) => { if (l.type === "error") console.log("[browser]", l.text.slice(0, 300)); };

fs.mkdirSync(outDir, { recursive: true });
const t0 = Date.now();
const serveUrl = await bundle({ entryPoint: path.resolve(opt("entry", "src/index.ts")), publicDir: path.resolve("public") });
const composition = await selectComposition({ serveUrl, id, inputProps, chromiumOptions, onBrowserLog, browserExecutable, timeoutInMilliseconds: 120000 });
console.log("bundle", ((Date.now() - t0) / 1000).toFixed(1) + "s");
for (const item of list.split(",")) {
  const [f, name] = item.split(":");
  const t1 = Date.now();
  await renderStill({ composition, serveUrl, frame: Number(f), output: path.join(outDir, `${name}.jpg`), imageFormat: "jpeg", jpegQuality: 92,
    scale, inputProps, chromiumOptions, onBrowserLog, browserExecutable, timeoutInMilliseconds: 600000 });
  console.log(name, "f" + f, ((Date.now() - t1) / 1000).toFixed(1) + "s");
}
