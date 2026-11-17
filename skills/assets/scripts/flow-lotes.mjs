#!/usr/bin/env bun
// Lotes de geração de imagens no Google Flow (Nano Banana) com a cota diária GLOBAL da fábrica.
// Regra do usuário: a cota é por PROMPT (20/dia) e cada prompt aceita até 24 imagens, então todo pedido
// sai com o máximo (24): 480 imagens/dia, nunca 20. Lote = 1 prompt = 1 cenário, com 24 variações
// de enquadramento para vários planos daquele cenário + reservas.
//
// uso (na pasta do projeto):
//   flow-lotes.mjs plan [--uso 8]          roteiro/beats.json + assets/flow/estilo.json -> assets/flow/lotes.json
//   flow-lotes.mjs proximo                 próximo lote pendente como argumentos do MCP (sai 3 se a cota do dia acabou)
//   flow-lotes.mjs registrar <lote> <dir>  conta o prompt na cota, move as imagens para assets/flow/<lote>/ e marca "gerado"
//   flow-lotes.mjs cota [--max-por-prompt N] [--max-prompts N]   mostra/ajusta a cota global
//   flow-lotes.mjs status                  resumo dos lotes
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const args = process.argv.slice(2);
const cmd = args[0];
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const COTA = process.env.VF_FLOW_COTA ?? path.join(os.homedir(), ".cache/vf/flow-cota.json");
const LOTES = "assets/flow/lotes.json";
const hoje = () => new Date().toLocaleDateString("sv-SE"); // AAAA-MM-DD no fuso local
const amanha = () => { const d = new Date(); d.setDate(d.getDate() + 1); return d.toLocaleDateString("sv-SE"); };

function lerCota() {
  let c = { max_prompts: 20, max_por_prompt: 24, dia: hoje(), prompts: 0, imagens: 0, historico: [] };
  try { c = { ...c, ...JSON.parse(fs.readFileSync(COTA, "utf8")) }; } catch {}
  if (c.dia !== hoje()) { c.historico = [...(c.historico ?? []), { dia: c.dia, prompts: c.prompts, imagens: c.imagens }].slice(-60); c.dia = hoje(); c.prompts = 0; c.imagens = 0; }
  return c;
}
const salvarCota = (c) => { fs.mkdirSync(path.dirname(COTA), { recursive: true }); fs.writeFileSync(COTA, JSON.stringify(c, null, 2)); };
const ler = (p, d) => { try { return JSON.parse(fs.readFileSync(p, "utf8")); } catch { return d; } };

if (cmd === "plan") {
  const uso = Number(opt("uso", 8)); // quantas das 24 imagens viram planos; o resto é reserva/alternativa
  const B = ler("roteiro/beats.json", null);
  if (!B) { console.error("roteiro/beats.json não encontrado"); process.exit(2); }
  const E = ler("assets/flow/estilo.json", null);
  if (!E) { console.error("assets/flow/estilo.json não encontrado: copie o template do plugin (skills/assets/templates/flow-estilo.json) e preencha a partir da style bible"); process.exit(2); }
  const cota = lerCota();
  // planos: cada item de `visual` (separado por ';') de cada beat; cenário = beat.cenario (obrigatório no motor estático)
  const planos = [];
  for (const b of B.beats ?? []) {
    const vs = String(b.visual ?? "").split(";").map((s) => s.trim()).filter(Boolean);
    vs.forEach((v, k) => planos.push({ id: `b${b.id}-${k + 1}`, beat: b.id, cenario: b.cenario ?? `beat-${b.id}`, visual: v }));
  }
  const cen = new Map();
  for (const p of planos) { if (!cen.has(p.cenario)) cen.set(p.cenario, []); cen.get(p.cenario).push(p); }
  const desc = E.cenarios ?? {};
  const semDesc = [...cen.keys()].filter((k) => !desc[k]);
  if (semDesc.length) { console.error(`cenários sem descrição em assets/flow/estilo.json > cenarios: ${semDesc.join(", ")}`); process.exit(2); }
  const antigos = Object.fromEntries((ler(LOTES, { lotes: [] }).lotes ?? []).map((l) => [l.id, l]));
  const lotes = [];
  for (const [k, ps] of cen) {
    const n = Math.ceil(ps.length / uso), per = Math.ceil(ps.length / n); // divide por igual (12 planos = 6+6, não 8+4)
    for (let i = 0; i < ps.length; i += per) {
      const grupo = ps.slice(i, i + per);
      const id = `${k}${n > 1 ? `-${i / per + 1}` : ""}`;
      const momentos = grupo.map((p) => p.visual).join(" | ");
      const prompt = [
        desc[k],
        `Moments to cover across the outputs (one per image, vary which one): ${momentos}.`,
        `Vary the framing across outputs: wide establishing shot, medium shot, close-up detail, low angle, over-the-shoulder, high angle.`,
        E.sufixo,
        E.negativos ? `Avoid: ${E.negativos}.` : "",
      ].filter(Boolean).join(" ");
      const old = antigos[id];
      lotes.push({ id, cenario: k, planos: grupo.map((p) => p.id), prompt, count: cota.max_por_prompt, ratio: E.ratio ?? "16:9", model: E.modelo ?? "nano-banana-pro",
        reference_images: (E.referencias ?? []).map((r) => path.resolve(r)), status: old?.status ?? "pendente", files: old?.files ?? [], gerado_em: old?.gerado_em });
    }
  }
  fs.mkdirSync(path.dirname(LOTES), { recursive: true });
  fs.writeFileSync(LOTES, JSON.stringify({ gerado: new Date().toISOString(), uso_por_lote: uso, planos: planos.length, lotes }, null, 2));
  const pend = lotes.filter((l) => l.status === "pendente").length;
  const dias = Math.ceil(pend / cota.max_prompts);
  console.log(`${planos.length} planos em ${cen.size} cenários -> ${lotes.length} lotes de ${cota.max_por_prompt} imagens (${lotes.length * cota.max_por_prompt} imagens). Pendentes: ${pend} (~${dias} dia(s) de cota). -> ${LOTES}`);
} else if (cmd === "proximo") {
  const L = ler(LOTES, null);
  if (!L) { console.error("rode `flow-lotes.mjs plan` antes"); process.exit(2); }
  const l = L.lotes.find((x) => x.status === "pendente");
  if (!l) { console.log(JSON.stringify({ fim: true, msg: "todos os lotes gerados" })); process.exit(0); }
  const c = lerCota();
  if (c.prompts >= c.max_prompts) {
    console.log(JSON.stringify({ cota_esgotada: true, usados: c.prompts, max: c.max_prompts, retomar_em: amanha() }));
    process.exit(3);
  }
  const out = path.resolve("assets/flow/_entrada", l.id);
  fs.mkdirSync(out, { recursive: true });
  // sempre o máximo de imagens por prompt: o prompt é o que custa, não a imagem
  console.log(JSON.stringify({ lote: l.id, cota_restante: c.max_prompts - c.prompts, mcp: "google-flow / flow_generate_image",
    args: { prompt: l.prompt, model: l.model, ratio: l.ratio, count: c.max_por_prompt, reference_images: l.reference_images, output_dir: out, auto_confirm: true } }, null, 2));
} else if (cmd === "registrar") {
  const [, id, dir] = args;
  const L = ler(LOTES, null);
  const l = L?.lotes.find((x) => x.id === id);
  if (!l || !dir) { console.error("uso: registrar <lote> <pasta-com-as-imagens>"); process.exit(2); }
  const imgs = fs.readdirSync(dir).filter((f) => /\.(png|jpe?g|webp)$/i.test(f)).sort();
  const dest = path.join("assets/flow", id);
  fs.mkdirSync(dest, { recursive: true });
  l.files = imgs.map((f, k) => {
    const to = path.join(dest, `${id}_${String(k + 1).padStart(2, "0")}${path.extname(f).toLowerCase()}`);
    fs.renameSync(path.join(dir, f), to);
    return to;
  });
  l.status = "gerado"; l.gerado_em = new Date().toISOString();
  const c = lerCota();
  c.prompts += 1; c.imagens += imgs.length;
  if (imgs.length < c.max_por_prompt) console.error(`aviso: o lote ${id} voltou com ${imgs.length} de ${c.max_por_prompt} imagens (se o Flow limita menos, ajuste: cota --max-por-prompt ${imgs.length})`);
  salvarCota(c);
  fs.writeFileSync(LOTES, JSON.stringify(L, null, 2));
  console.log(`${id}: ${imgs.length} imagens -> ${dest} · cota do dia ${c.prompts}/${c.max_prompts} prompts, ${c.imagens} imagens`);
} else if (cmd === "cota") {
  const c = lerCota();
  if (opt("max-por-prompt")) c.max_por_prompt = Number(opt("max-por-prompt"));
  if (opt("max-prompts")) c.max_prompts = Number(opt("max-prompts"));
  salvarCota(c);
  console.log(`cota ${c.dia}: ${c.prompts}/${c.max_prompts} prompts, ${c.imagens} imagens (máx ${c.max_por_prompt}/prompt = ${c.max_prompts * c.max_por_prompt}/dia) · ${COTA}`);
} else if (cmd === "status") {
  const L = ler(LOTES, { lotes: [] });
  const by = (s) => L.lotes.filter((l) => l.status === s).length;
  console.log(`lotes: ${L.lotes.length} · pendente ${by("pendente")} · gerado ${by("gerado")} · curado ${by("curado")}`);
  for (const l of L.lotes) console.log(`  ${l.status.padEnd(8)} ${l.id.padEnd(24)} ${l.planos.length} planos ${l.files.length ? `${l.files.length} imagens` : ""}`);
} else {
  console.error("comandos: plan | proximo | registrar <lote> <dir> | cota | status");
  process.exit(2);
}
