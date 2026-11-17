// Luz dinâmica de direção (shader de pós, postprocessing 6.37): a luz como ferramenta narrativa, não só física.
//   spot     destaque: escurece e dessatura a periferia e aquece/levanta o assunto (personagem que fala, prop-chave)
//   rays     god rays em espaço de tela a partir de uma fonte (janela, sol, lâmpada, revelação)
//   exposure stops (+0,3 = cena se abre; -0,4 = tensão)
//   temp     -1 frio … +1 quente (mudança de clima/ato sem trocar o rig do mundo)
// Os cues são keyframes por frame com entrada/saída suave; vários se somam. Coloque o efeito ANTES do bloom
// e do tone mapping (trabalha em linear HDR, então o que ele ilumina também floresce).
//
//   const cues: LightCue[] = [
//     { from: 120, to: 210, spot: { at: [0.2, 1.6, -3], r: 0.32, k: 0.7 } },   // destaca quem fala (posição no mundo)
//     { from: 300, to: 420, rays: { at: [0.75, 0.12], k: 0.8 }, exposure: 0.25, temp: 0.4 }, // janela, ponto de tela (uv)
//   ];
//   no Inner: applyCues(fx.light, cueAt(cues, f), cam)
import { Effect, BlendFunction } from "postprocessing";
import * as THREE from "three";

type V3 = [number, number, number];
type At = V3 | [number, number]; // mundo (3) ou uv de tela (2: x 0..1 da esquerda, y 0..1 de CIMA)
export type LightCue = {
  from: number; to: number;      // frames
  ease?: number;                 // frames de entrada/saída (padrão 8)
  spot?: { at: At; r?: number; k?: number };   // r: raio em alturas de quadro (0,3); k: força 0..1 (0,6)
  rays?: { at: At; k?: number; decay?: number; r?: number }; // k: força (0,6); r: raio da fonte em alturas de quadro (0,2)
  exposure?: number; temp?: number;
};
export type LightState = { spot?: { at: At; r: number; k: number }; rays?: { at: At; k: number; decay: number; r: number }; exposure: number; temp: number };

const smooth = (x: number) => { const t = Math.min(1, Math.max(0, x)); return t * t * (3 - 2 * t); };

/** Estado da luz no frame f: pesos suaves por cue; spot/rays usam o cue de maior peso (alvos não se misturam). */
export function cueAt(cues: LightCue[], f: number): LightState {
  const s: LightState = { exposure: 0, temp: 0 };
  let ws = 0, wr = 0;
  for (const c of cues) {
    const e = c.ease ?? 8;
    const w = smooth((f - c.from) / e) * smooth((c.to - f) / e);
    if (w <= 0) continue;
    s.exposure += (c.exposure ?? 0) * w;
    s.temp += (c.temp ?? 0) * w;
    if (c.spot && w > ws) { ws = w; s.spot = { at: c.spot.at, r: c.spot.r ?? 0.3, k: (c.spot.k ?? 0.6) * w }; }
    if (c.rays && w > wr) { wr = w; s.rays = { at: c.rays.at, k: (c.rays.k ?? 0.6) * w, decay: c.rays.decay ?? 0.96, r: c.rays.r ?? 0.2 }; }
  }
  return s;
}

const frag = /* glsl */ `
uniform vec2 lcSpot; uniform float lcSpotR, lcSpotK, lcAspect;
uniform vec2 lcRay; uniform float lcRayK, lcRayDecay, lcRayR;
uniform float lcExp, lcTemp;

float lcLuma(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }

void mainImage(const in vec4 inputColor, const in vec2 uv, out vec4 outputColor) {
  vec3 c = inputColor.rgb;
  // god rays: marcha da posição até a fonte acumulando só o que é brilhante (céu, janela, lâmpada)
  if (lcRayK > 0.001) {
    // dither por pixel no ponto de partida: sem faixas com poucas amostras
    float j = fract(sin(dot(uv * 913.0, vec2(12.9898, 78.233))) * 43758.5453);
    vec2 d = (uv - lcRay) * 0.6 / 32.0;
    vec2 p = uv - d * j; float w = 1.0; vec3 acc = vec3(0.0); float wsum = 0.0;
    for (int i = 0; i < 32; i++) {
      p -= d;
      vec3 s = texture2D(inputBuffer, clamp(p, 0.0, 1.0)).rgb;
      float src = 1.0 - smoothstep(lcRayR * 0.6, lcRayR, length((p - lcRay) * vec2(lcAspect, 1.0)));
      acc += max(s - 1.0, 0.0) * src * w;                 // só a fonte HDR do cue (janela, sol), não outros brilhos
      wsum += w; w *= lcRayDecay;
    }
    float fall = 1.0 - smoothstep(0.0, 0.9, length((uv - lcRay) * vec2(lcAspect, 1.0)));
    c += acc / wsum * lcRayK * 1.6 * fall * vec3(1.0, 0.93, 0.8);
  }
  // destaque: periferia mais escura e menos saturada; assunto levemente mais quente e claro
  if (lcSpotK > 0.001) {
    vec2 q = (uv - lcSpot) * vec2(lcAspect, 1.0);
    float m = 1.0 - smoothstep(lcSpotR * 0.55, lcSpotR * 1.35, length(q));
    float y = lcLuma(c);
    vec3 dim = mix(c, vec3(y), 0.35 * lcSpotK) * (1.0 - 0.45 * lcSpotK);
    vec3 lift = c * (1.0 + 0.12 * lcSpotK) * vec3(1.03, 1.0, 0.96);
    c = mix(dim, lift, m);
  }
  c *= exp2(lcExp);
  c *= mix(vec3(1.0), lcTemp > 0.0 ? vec3(1.08, 1.0, 0.86) : vec3(0.9, 0.98, 1.1), abs(lcTemp));
  outputColor = vec4(c, inputColor.a);
}`;

export class LightCueEffect extends Effect {
  constructor() {
    super("LightCueEffect", frag, {
      blendFunction: BlendFunction.NORMAL,
      uniforms: new Map<string, THREE.Uniform>([
        ["lcSpot", new THREE.Uniform(new THREE.Vector2(0.5, 0.5))], ["lcSpotR", new THREE.Uniform(0.3)], ["lcSpotK", new THREE.Uniform(0)],
        ["lcAspect", new THREE.Uniform(9 / 16)],
        ["lcRay", new THREE.Uniform(new THREE.Vector2(0.5, 0.1))], ["lcRayK", new THREE.Uniform(0)], ["lcRayDecay", new THREE.Uniform(0.96)], ["lcRayR", new THREE.Uniform(0.2)],
        ["lcExp", new THREE.Uniform(0)], ["lcTemp", new THREE.Uniform(0)],
      ]),
    });
  }
  setSize(w: number, h: number) { this.uniforms.get("lcAspect")!.value = w / Math.max(h, 1); }
}

const tmp = new THREE.Vector3();
/** mundo -> uv (y de baixo, como o shader); uv de tela (y de cima) -> uv do shader */
function toUv(at: At, cam: THREE.Camera, out: THREE.Vector2) {
  if (at.length === 2) return out.set(at[0], 1 - at[1]);
  tmp.set(at[0], at[1], at[2]).project(cam);
  return out.set(tmp.x * 0.5 + 0.5, tmp.y * 0.5 + 0.5);
}

export function applyCues(fx: LightCueEffect, s: LightState, cam: THREE.Camera) {
  const u = fx.uniforms;
  u.get("lcExp")!.value = s.exposure;
  u.get("lcTemp")!.value = Math.max(-1, Math.min(1, s.temp));
  u.get("lcSpotK")!.value = s.spot?.k ?? 0;
  if (s.spot) { toUv(s.spot.at, cam, u.get("lcSpot")!.value); u.get("lcSpotR")!.value = s.spot.r; }
  u.get("lcRayK")!.value = s.rays?.k ?? 0;
  if (s.rays) { toUv(s.rays.at, cam, u.get("lcRay")!.value); u.get("lcRayDecay")!.value = s.rays.decay; u.get("lcRayR")!.value = s.rays.r; }
}
