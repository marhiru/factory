// Bun importa a timeline TypeScript real; não tenta analisar código com regex.
// Uso: bun scripts/cartoon-plan.mjs [src/videos/cartoon/story.ts] [out/cartoon-plan.json]
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const [entry = "src/videos/cartoon/story.ts", output = "out/cartoon-plan.json"] = process.argv.slice(2);
const { SHOTS, XFADE = {}, FLASH_AT, FPS, TOTAL } = await import(pathToFileURL(path.resolve(entry)).href);
if (!Number.isInteger(FPS) || FPS <= 0 || !Number.isInteger(TOTAL) || TOTAL <= 0 || !Array.isArray(SHOTS) || !SHOTS.length) {
  throw new Error("timeline sem FPS/TOTAL/SHOTS válidos");
}
let end = 0;
const shots = SHOTS.map(({ id, a, b }) => {
  if (!Number.isInteger(a) || !Number.isInteger(b) || a !== end || b <= a || b > TOTAL) {
    throw new Error(`plano ${id}: intervalo inválido ou cobertura descontínua`);
  }
  end = b;
  return { id, a, b };
});
if (end !== TOTAL) throw new Error("SHOTS não cobre TOTAL");
const xfade = shots.filter(({ id }) => XFADE[id]).map(({ id, a }) => {
  const n = XFADE[id];
  if (!Number.isInteger(n) || n <= 0) throw new Error(`xfade inválido: ${id}`);
  return [Math.max(0, Math.floor(a - n / 2)), Math.min(TOTAL - 1, Math.ceil(a + n / 2) - 1)];
});
const plan = { shots, xfade, planned: [] };
if (FLASH_AT !== undefined) {
  if (!Number.isInteger(FLASH_AT) || FLASH_AT < 0 || FLASH_AT + 4 > TOTAL) throw new Error("FLASH_AT inválido");
  plan.flash = [FLASH_AT, FLASH_AT + 3];
}
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(plan, null, 2) + "\n");
console.log(`${shots.length} planos, ${xfade.length} dissolves; ${output} (frames a ${FPS} fps)`);
