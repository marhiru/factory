#!/usr/bin/env bun
// Exporta os packs de sprites (registry Canvas2D) para public/sprites/<pack>/*.png + sprite.json.
// uso: bun export-sprites.mjs <entry: sprites/index.ts> <saida: public/sprites> [pack1,pack2...]
// O entry registra <SpritesRoot packs={PACKS} /> (templates/sprites/SpriteExport.tsx); metadados vêm da composição.
import { bundle } from "@remotion/bundler";
import { renderStill, selectComposition } from "@remotion/renderer";
import fs from "node:fs";
import path from "node:path";

const [entry, outRoot, only] = process.argv.slice(2);
if (!entry || !outRoot) { console.error("uso: export-sprites.mjs sprites/index.ts public/sprites [packs]"); process.exit(2); }
const t0 = Date.now();
// REMOTION_BROWSER: caminho do chrome-headless-shell já baixado pelo CLI (evita novo download)
const browserExecutable = process.env.REMOTION_BROWSER || null;
const serveUrl = await bundle({ entryPoint: path.resolve(entry) });
const first = await selectComposition({ serveUrl, id: "Sprites", inputProps: {}, browserExecutable });
const names = only ? only.split(",") : first.props.names;
const chromiumOptions = { gl: "angle" };
let n = 0;
for (const name of names) {
  const composition = await selectComposition({ serveUrl, id: "Sprites", inputProps: { pack: name }, browserExecutable });
  const p = composition.props.meta;
  const dir = path.join(outRoot, p.name);
  fs.mkdirSync(dir, { recursive: true });
  for (let i = 0; i < p.items.length; i++) {
    await renderStill({ composition, serveUrl, frame: i, output: path.join(dir, `${p.items[i].file}.png`), imageFormat: "png", inputProps: { pack: p.name }, chromiumOptions, browserExecutable });
    n++;
  }
  const states = {};
  for (const it of p.items) {
    const s = (states[it.state] ??= { frames: [] });
    if (it.role === "blink") s.blink = `${it.file}.png`;
    else s.frames.push(`${it.file}.png`);
    if (p.fps?.[it.state]) s.fps = p.fps[it.state];
  }
  fs.writeFileSync(path.join(dir, "sprite.json"), JSON.stringify({ w: Math.round(p.w * p.scale), h: Math.round(p.h * p.scale), anchor: p.anchor ?? [0.5, 0.975], states }, null, 2));
  console.log(`${p.name}: ${p.items.length} PNG`);
}
console.log(`${n} sprites em ${((Date.now() - t0) / 1000).toFixed(1)} s -> ${outRoot}`);
