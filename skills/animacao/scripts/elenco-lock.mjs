#!/usr/bin/env bun
// Trava de identidade do elenco (QA etapa 3b): uma correção de posição/pose não pode trocar o personagem.
//   bun elenco-lock.mjs snapshot            grava assets/elenco.lock.json (hash de cada PNG e do sprite.json por pack)
//   bun elenco-lock.mjs check [--pedido posicao|pose|novo-estado]   compara o estado atual com a trava
// Regras do check:
//   posicao      (padrão) nenhum arquivo de nenhum pack pode mudar: a correção é só no layout (cenas/edl)
//   novo-estado  pode ACRESCENTAR estados/frames a um pack; frames existentes não podem mudar
//   pose         idem novo-estado (pose nova = estado novo do mesmo pack, gerado da mesma referência)
// Sai com código 1 se a regra foi violada, listando o que mudou. Packs `fx-*` e `luz-*` são ignorados.
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const ROOT = "public/sprites";
const LOCK = "assets/elenco.lock.json";
const [cmd = "check", ...rest] = process.argv.slice(2);
const pedido = rest.includes("--pedido") ? rest[rest.indexOf("--pedido") + 1] : "posicao";

function scan() {
  const out = {};
  if (!fs.existsSync(ROOT)) return out;
  for (const pack of fs.readdirSync(ROOT).sort()) {
    if (/^(fx|luz)-/.test(pack)) continue;
    const dir = path.join(ROOT, pack);
    if (!fs.statSync(dir).isDirectory()) continue;
    const files = {};
    for (const f of fs.readdirSync(dir).sort()) {
      if (!/\.(png|json)$/.test(f)) continue;
      files[f] = createHash("sha256").update(fs.readFileSync(path.join(dir, f))).digest("hex").slice(0, 16);
    }
    let states = [];
    try { states = Object.keys(JSON.parse(fs.readFileSync(path.join(dir, "sprite.json"), "utf8")).states ?? {}); } catch {}
    out[pack] = { files, states };
  }
  return out;
}

if (cmd === "snapshot") {
  fs.mkdirSync(path.dirname(LOCK), { recursive: true });
  const s = scan();
  fs.writeFileSync(LOCK, JSON.stringify({ criado: new Date().toISOString(), packs: s }, null, 1));
  console.log(`${LOCK}: ${Object.keys(s).length} pack(s) travados`);
  process.exit(0);
}

if (!fs.existsSync(LOCK)) { console.error(`sem ${LOCK}: rode "snapshot" quando o elenco for aprovado`); process.exit(2); }
const lock = JSON.parse(fs.readFileSync(LOCK, "utf8")).packs;
const now = scan();
const erros = [], avisos = [];
for (const [pack, old] of Object.entries(lock)) {
  const cur = now[pack];
  if (!cur) { erros.push(`${pack}: pack sumiu (personagem trocado/removido)`); continue; }
  for (const [f, h] of Object.entries(old.files)) {
    if (f === "sprite.json") continue; // estados novos mudam o json; checado pelos estados abaixo
    if (!(f in cur.files)) erros.push(`${pack}/${f}: frame removido`);
    else if (cur.files[f] !== h) erros.push(`${pack}/${f}: frame redesenhado (identidade pode ter mudado)`);
  }
  for (const st of old.states) if (!cur.states.includes(st)) erros.push(`${pack}: estado "${st}" removido`);
  const novos = Object.keys(cur.files).filter((f) => !(f in old.files));
  if (novos.length) (pedido === "posicao" ? erros : avisos).push(`${pack}: ${novos.length} arquivo(s) novo(s): ${novos.slice(0, 6).join(", ")}${novos.length > 6 ? "…" : ""}`);
}
for (const pack of Object.keys(now)) if (!(pack in lock)) (pedido === "posicao" ? erros : avisos).push(`${pack}: pack novo (é um personagem novo de verdade ou um substituto?)`);

for (const a of avisos) console.log(`AVISO ${a} — confira antes × depois lado a lado`);
if (erros.length) {
  for (const e of erros) console.log(`FAIL  ${e}`);
  console.log(`\nidentidade REPROVADA para pedido "${pedido}": a correção deve mexer só no layout/pose do mesmo pack (QA 3b)`);
  process.exit(1);
}
console.log(`identidade OK (${Object.keys(lock).length} pack(s), pedido "${pedido}")`);
