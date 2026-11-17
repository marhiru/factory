// Motor de FX por shader (GLSL) da fábrica: um card voltado para a câmera com ShaderMaterial aditivo em HDR.
// Valores > 1 passam do limiar do bloom (Bridge.tsx: luminanceThreshold 1.0), então o brilho espalha sozinho
// e desfoca junto com a fonte (DOF antes do bloom). Tudo é função de (uTime, uSeed): determinístico por frame.
//
// Tipos (uKind):
//   glow    halo de energia/luz com núcleo quente e borda colorida (magia, lâmpada, ideia)
//   energy  chama/plasma com fbm em domínio deformado (fogo cartoon, aura, poder)
//   ring    onda de choque: anel que expande com borda quente e rastro (impacto, revelação)
//   sparkle estrelas de 4 pontas cintilando (brilho, limpeza, "ding")
//   beam    feixe/god ray volumétrico de cima para baixo (holofote, revelação divina, luz pela janela)
//   dust    nuvem de poeira/fumaça ILUMINADA (lado da luz quente, sombra fria) + destroços em parábola:
//           desabamento, explosão, impacto no chão, e "wipe" de transição (a nuvem enche o quadro e troca de lugar).
//           Não é aditivo: ocupa espaço e esconde o fundo (blend normal pré-multiplicado).
//
// Uso (world-space, dentro do World/stage):
//   const fx = new ShaderFx("ring", { color: "#ffd27a", intensity: 3 });  scene.add(fx.mesh);
//   por frame: fx.set(tSegundosDesdeOInicio, cam);  fx.mesh.position.set(...); fx.mesh.scale.setScalar(2);
// Para o mygo: renderize a composição FxSheet (Root.tsx) em PNG com alpha e converta com animacao/scripts/fx-pack.mjs.
import * as THREE from "three";

export type FxKind = "glow" | "energy" | "ring" | "sparkle" | "beam" | "dust";
const KIND: Record<FxKind, number> = { glow: 0, energy: 1, ring: 2, sparkle: 3, beam: 4, dust: 5 };

export type FxOpts = {
  color?: string;      // cor principal (borda); o núcleo vai para o branco quente
  color2?: string;     // cor secundária (rastro/fumaça); padrão: color escurecida
  intensity?: number;  // 1 = no limiar do bloom; 2-4 = brilho forte
  duration?: number;   // s: one-shots (ring, sparkle) usam para envelope; loops ignoram
  seed?: number;
  loop?: boolean;      // glow/energy/beam: true; ring/sparkle: false
};

const vert = /* glsl */ `
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`;

export const FX_GLSL = /* glsl */ `
precision highp float;
varying vec2 vUv;
uniform float uTime, uDur, uSeed, uInt, uLoop, uSheet;
uniform vec2 uLight; // direção da luz-chave no card (x direita, y cima), para sombrear a poeira
uniform int uKind;
uniform vec3 uCol, uCol2;

float h21(vec2 p) { p = fract(p * vec2(123.34, 456.21) + uSeed * 0.137); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float vnoise(vec2 p) {
  vec2 i = floor(p), f = fract(p); vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(h21(i), h21(i + vec2(1, 0)), u.x), mix(h21(i + vec2(0, 1)), h21(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) { float a = 0.5, s = 0.0; for (int i = 0; i < 5; i++) { s += a * vnoise(p); p = p * 2.03 + vec2(1.7, 9.2); a *= 0.5; } return s; }
// rampa de cor: escuro/secundária -> cor -> branco quente (o núcleo "estoura" para o bloom)
vec3 ramp(float x) {
  vec3 hot = vec3(1.0, 0.97, 0.9);
  return x < 0.5 ? mix(uCol2 * 0.6, uCol, smoothstep(0.0, 0.5, x)) : mix(uCol, hot, smoothstep(0.5, 1.0, x));
}
float env(float t) { // envelope de one-shot: ataque rápido, decaimento suave
  if (uLoop > 0.5) return 1.0;
  float k = clamp(t / max(uDur, 1e-3), 0.0, 1.0);
  return smoothstep(0.0, 0.06, k) * (1.0 - smoothstep(0.55, 1.0, k));
}

vec4 glow(vec2 p, float t) {
  float r = length(p);
  float flick = 0.9 + 0.1 * vnoise(vec2(t * 6.0, uSeed));
  float core = exp(-r * r * 60.0);
  float halo = exp(-r * 5.5) * 0.55;
  float rays = pow(max(0.0, cos(atan(p.y, p.x) * 6.0 + t * 0.7)), 18.0) * exp(-r * 4.0) * 0.35;
  float a = (core + halo + rays) * flick;
  return vec4(ramp(clamp(core * 1.4 + halo * 0.6, 0.0, 1.0)) * a, a);
}

vec4 energy(vec2 p, float t) {
  // chama cartoon: corpo em gota, línguas que sobem (ruído rolando para cima e esticado em y), borda recortada
  vec2 q = vec2(p.x, p.y + 0.38);                            // base perto do fundo do card
  float y = clamp(q.y / 0.8, 0.0, 1.0);                      // 0 base .. 1 topo
  vec2 w = vec2(fbm(vec2(q.x * 3.0, q.y * 1.4 - t * 1.8)), fbm(vec2(q.x * 3.0 + 4.1, q.y * 1.4 - t * 2.1)));
  float n = fbm(vec2(q.x * 4.5 + (w.x - 0.5) * 1.2, q.y * 2.2 - t * 3.2));
  float width = mix(0.26, 0.03, pow(y, 1.25)) * (0.7 + 0.6 * n); // afina para cima; o ruído recorta as línguas
  float body = 1.0 - smoothstep(width * 0.6, width, abs(q.x + (w.y - 0.5) * 0.18 * y));
  float bulb = 1.0 - smoothstep(0.12, 0.2, length(vec2(q.x, (q.y - 0.12) * 1.25)) + (n - 0.5) * 0.08); // base redonda (gota)
  body = max(body * smoothstep(0.02, 0.2, q.y), bulb);        // emenda suave entre gota e corpo
  float f = body * (1.0 - smoothstep(0.55, 1.0, y + (n - 0.5) * 0.6));
  f = clamp(f * (0.75 + n * 0.6) - y * 0.15, 0.0, 1.0);
  float edge = smoothstep(0.08, 0.16, f);                    // recorte cartoon
  float heat = clamp(f * 0.95 - y * 0.55, 0.0, 1.0);         // núcleo quente só na base: o corpo fica na cor
  return vec4(ramp(heat) * edge, edge);
}

vec4 ring(vec2 p, float t) {
  float k = clamp(t / max(uDur, 1e-3), 0.0, 1.0);
  float R = 0.08 + 0.42 * (1.0 - pow(1.0 - k, 3.0));       // expansão ease-out
  float w = mix(0.07, 0.012, k);
  float r = length(p);
  vec2 dir = p / max(r, 1e-4);                               // ruído periódico no ângulo: sem costura em ±π
  float wob = (vnoise(dir * 2.5 + uSeed) - 0.5) * 0.03;
  float band = exp(-pow((r - R + wob) / w, 2.0));
  float trail = smoothstep(R, R * 0.4, r) * (1.0 - k) * 0.15;
  float flash = exp(-r * r * 40.0) * (1.0 - smoothstep(0.0, 0.18, k)) * 1.5;
  float a = (band + trail + flash) * (1.0 - smoothstep(0.7, 1.0, k));
  return vec4(ramp(clamp(band * 0.9 + flash, 0.0, 1.0)) * a, a);
}

vec4 sparkle(vec2 p, float t) {
  vec3 acc = vec3(0.0); float al = 0.0;
  for (int i = 0; i < 7; i++) {
    float fi = float(i);
    vec2 c = (vec2(h21(vec2(fi, 1.0)), h21(vec2(fi, 7.0))) - 0.5) * 0.75;
    float ph = h21(vec2(fi, 3.0));
    float life = clamp((t / max(uDur, 1e-3)) * 1.6 - ph * 0.6, 0.0, 1.0);
    float s = sin(life * 3.14159);                           // nasce, cresce, some
    vec2 d = (p - c) / (0.05 + 0.1 * h21(vec2(fi, 5.0)));
    float ang = t * 1.5 + fi;
    d = mat2(cos(ang), -sin(ang), sin(ang), cos(ang)) * d;
    float star = max(0.0, 1.0 - abs(d.x * d.y) * 18.0 - length(d) * 0.9);
    star = pow(star, 3.0) + exp(-dot(d, d) * 6.0) * 0.6;
    acc += ramp(clamp(star, 0.0, 1.0)) * star * s; al += star * s;
  }
  return vec4(acc, clamp(al, 0.0, 1.0));
}

vec4 beam(vec2 p, float t) {
  float y = 0.5 - p.y;                                       // 0 no topo -> 1 embaixo
  float wHalf = mix(0.06, 0.32, y);                          // cone que abre para baixo
  float side = 1.0 - smoothstep(wHalf * 0.55, wHalf, abs(p.x));
  float shafts = 0.65 + 0.35 * fbm(vec2(p.x / wHalf * 3.0, t * 0.25));
  // poeira: um grão redondo por célula, derivando devagar e cintilando (nada de blocos de ruído)
  vec2 g = p * 26.0 + vec2(0.0, t * 0.35); vec2 id = floor(g), fp = fract(g) - 0.5;
  vec2 off = (vec2(h21(id), h21(id + 3.1)) - 0.5) * 0.6;
  float dust = smoothstep(0.09, 0.0, length(fp - off)) * step(0.72, h21(id + 7.7)) * (0.5 + 0.5 * sin(t * 3.0 + h21(id) * 6.28)) * 2.0;
  float fall = smoothstep(0.0, 0.08, y) * (1.0 - smoothstep(0.75, 1.0, y));
  float a = side * fall * (shafts * 0.45 + dust * side);
  return vec4(ramp(0.55 + 0.4 * shafts * (1.0 - y)) * a, a * 0.8);
}


float dustD(vec2 p, float k) {
  // 10 bolhas de tamanhos variados que sobem e crescem; cada uma é uma "couve-flor" (fbm por bolha)
  float d = 0.0;
  for (int i = 0; i < 10; i++) {
    float fi = float(i);
    float hx = h21(vec2(fi, 2.0)), hy = h21(vec2(fi, 9.0)), hr = h21(vec2(fi, 4.0));
    vec2 c = vec2((hx - 0.5) * (0.2 + 0.35 * k), -0.32 + k * (0.15 + 0.4 * hy) + hy * 0.12);
    float r = (0.05 + 0.08 * hr) * (0.5 + 1.1 * sqrt(k));
    vec2 q = (p - c) / r;
    float bump = fbm(q * 1.3 + fi * 3.7 + vec2(0.0, -k * 2.0)) - 0.5;
    d = max(d, 1.0 - smoothstep(0.75, 1.0, length(q) - bump * 0.55));
  }
  return d;
}

vec4 dust(vec2 p, float t) {
  float k = clamp(t / max(uDur, 1e-3), 0.0, 1.0);
  float d = dustD(p, k);
  vec2 L = normalize(uLight) * 0.035;
  // lado voltado para a luz = claro: andar na direção da luz sai da nuvem (densidade cai)
  // claro por padrão; sombra só no lado oposto à luz (andar contra a luz sai da nuvem)
  float lit = clamp(0.8 + (d - dustD(p + L, k)) * 2.0 - (1.0 - dustD(p - L * 1.5, k)) * 1.2, 0.0, 1.0);
  float a = smoothstep(0.1, 0.3, d) * (1.0 - smoothstep(0.7, 1.0, k));
  vec3 col = mix(uCol2, uCol, smoothstep(0.35, 0.65, lit));          // two-tone (toon), sem degradê lavado
  // destroços: pedaços escuros em parábola, saem no começo
  float deb = 0.0;
  for (int i = 0; i < 10; i++) {
    float fi = float(i);
    vec2 v = vec2((h21(vec2(fi, 11.0)) - 0.5) * 1.4, 0.9 + 0.8 * h21(vec2(fi, 13.0)));
    vec2 q = vec2(0.0, -0.3) + v * t - vec2(0.0, 1.6 * t * t);
    float s = 0.008 + 0.01 * h21(vec2(fi, 17.0));
    deb += smoothstep(s, s * 0.4, length(p - q)) * (1.0 - smoothstep(0.4, 0.9, k));
  }
  vec3 rgb = mix(col, uCol2 * 0.35, clamp(deb, 0.0, 1.0));
  float alpha = clamp(max(a, deb), 0.0, 1.0);
  return vec4(rgb * alpha, alpha);
}

void main() {
  vec2 p = vUv - 0.5;
  float t = uTime;
  vec4 c = uKind == 0 ? glow(p, t) : uKind == 1 ? energy(p, t) : uKind == 2 ? ring(p, t) : uKind == 3 ? sparkle(p, t) : uKind == 4 ? beam(p, t) : dust(p, t);
  float e = env(t) * smoothstep(0.5, 0.4, max(abs(p.x), abs(p.y))); // some antes da borda do card: nunca aparece o quadrado
  // aditivo pré-multiplicado: rgb já vem multiplicado pela cobertura; uInt empurra o núcleo acima de 1 (bloom)
  vec3 rgb = c.rgb * uInt * e; float a = clamp(c.a * e, 0.0, 1.0);
  // modo sheet (PNG para o mygo, que compõe com alpha normal): luz aditiva vira cobertura, sem estourar
  // só luz: a cobertura vem do brilho, então partes escuras (rastro, fumaça fraca) nunca escurecem o fundo
  if (uSheet > 0.5 && uKind != 5) { rgb = min(rgb, vec3(1.0)); a = max(rgb.r, max(rgb.g, rgb.b)); rgb = a > 1e-4 ? rgb / a : rgb; }
  else if (uSheet > 0.5) { rgb = a > 1e-4 ? min(rgb / a, vec3(1.0)) : rgb; } // poeira: alpha real, cor sem pré-multiplicar
  gl_FragColor = vec4(rgb, a);
  #include <colorspace_fragment>
}`;

const lin = (hex: string) => new THREE.Color(hex).convertSRGBToLinear();

export class ShaderFx {
  readonly mesh: THREE.Mesh<THREE.PlaneGeometry, THREE.ShaderMaterial>;
  readonly uniforms: Record<string, THREE.IUniform>;
  constructor(kind: FxKind, o: FxOpts = {}) {
    const col = lin(o.color ?? "#ffcf6a");
    const col2 = o.color2 ? lin(o.color2) : kind === "dust" ? lin("#7d6a86") : col.clone().multiplyScalar(0.7); // poeira: sombra fria por padrão
    this.uniforms = {
      uTime: { value: 0 }, uDur: { value: o.duration ?? 0.8 }, uSeed: { value: o.seed ?? 1 }, uInt: { value: o.intensity ?? 2 },
      uLight: { value: new THREE.Vector2(0.6, 0.8) }, uSheet: { value: 0 }, uLoop: { value: (o.loop ?? (kind === "glow" || kind === "energy" || kind === "beam")) ? 1 : 0 },
      uKind: { value: KIND[kind] }, uCol: { value: new THREE.Vector3(col.r, col.g, col.b) }, uCol2: { value: new THREE.Vector3(col2.r, col2.g, col2.b) },
    };
    const mat = new THREE.ShaderMaterial({
      vertexShader: vert, fragmentShader: FX_GLSL, uniforms: this.uniforms,
      transparent: true, depthWrite: false, depthTest: true, toneMapped: false,
      blending: THREE.CustomBlending, blendEquation: THREE.AddEquation, blendSrc: THREE.OneFactor,
      blendDst: kind === "dust" ? THREE.OneMinusSrcAlphaFactor : THREE.OneFactor, // poeira cobre; luz soma
    });
    this.mesh = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), mat);
    this.mesh.renderOrder = 10; // depois do mundo opaco e dos cards
    this.mesh.frustumCulled = false;
    this.mesh.layers.enable(0);
  }
  /** t em segundos desde o início do FX; se cam vier, o card encara a câmera (billboard) */
  set(t: number, cam?: THREE.Camera) {
    this.uniforms.uTime.value = t;
    if (cam) this.mesh.quaternion.copy(cam.quaternion);
    this.mesh.visible = this.uniforms.uLoop.value > 0.5 || (t >= 0 && t <= this.uniforms.uDur.value);
  }
  dispose() { this.mesh.geometry.dispose(); this.mesh.material.dispose(); }
}
