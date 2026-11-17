// Calor no MATERIAL do asset (não sprite por cima): o objeto esquenta e passa a emitir luz.
// Medido na referência (trilho dilatando no verão): roxo → magenta → laranja → amarelo-branco ao longo do
// beat, com emissivo > 1 no pico para o bloom espalhar, e a cor anda junto com a fala ("em maio… em julho…").
//   heatOn(mesh.material as THREE.MeshStandardMaterial, cueHeat(f, 300, 420));  // 0..1 entre os frames
import * as THREE from "three";

const STOPS: [number, string][] = [[0, "#3a1c6b"], [0.3, "#b02bd6"], [0.55, "#ff4fa0"], [0.8, "#ffb23a"], [1, "#fff4c8"]];
const C = STOPS.map(([k, h]) => [k, new THREE.Color(h).convertSRGBToLinear()] as const);
const tmp = new THREE.Color();

/** cor linear da rampa de calor para h em 0..1 */
export function heatColor(h: number, out = tmp) {
  const x = Math.min(1, Math.max(0, h));
  for (let i = 1; i < C.length; i++) {
    if (x <= C[i][0]) { const [k0, c0] = C[i - 1], [k1, c1] = C[i]; return out.copy(c0).lerp(c1, (x - k0) / (k1 - k0)); }
  }
  return out.copy(C[C.length - 1][1]);
}

/** progresso suave 0..1 entre dois frames (ease in-out) */
export const cueHeat = (f: number, from: number, to: number) => { const t = Math.min(1, Math.max(0, (f - from) / Math.max(1, to - from))); return t * t * (3 - 2 * t); };

/** aplica o calor: emissivo na cor da rampa; intensidade passa de 1 (bloom) só no último terço */
export function heatOn(m: THREE.MeshStandardMaterial | THREE.MeshLambertMaterial, h: number) {
  m.emissive.copy(heatColor(h));
  m.emissiveIntensity = h < 0.05 ? 0 : 0.4 + 2.2 * h * h;
  m.toneMapped = true;
}
