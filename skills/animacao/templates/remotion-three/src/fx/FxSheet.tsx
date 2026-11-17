// Composições de FX por shader:
//   FxSheet  um FX centralizado em fundo transparente, para virar pack de sprites do mygo:
//            remotion render src/index.ts FxSheet out/fx/ring --sequence --image-format png \
//              --props '{"kind":"ring","color":"#ffd27a","duration":0.8}'
//            bun ${PLUGIN}/skills/animacao/scripts/fx-pack.mjs out/fx/ring public/sprites/fx-ring --smooth --scale 1
//   FxLab    folha de conferência: os 5 FX sobre um mundo simples + luz dinâmica (spot e god rays) ligando/desligando
import React, { useMemo } from "react";
import * as THREE from "three";
import { AbsoluteFill, getInputProps, useCurrentFrame, useVideoConfig } from "remotion";
import { ThreeCanvas } from "@remotion/three";
import { useThree } from "@react-three/fiber";
import { EffectComposer } from "@react-three/postprocessing";
import { BlendFunction, BloomEffect, ToneMappingEffect, ToneMappingMode } from "postprocessing";
import { ShaderFx, type FxKind } from "./shaderfx";
import { LightCueEffect, applyCues, cueAt, type LightCue } from "./lightcue";
import { heatOn, cueHeat } from "./heat";

type SheetProps = { kind?: FxKind; color?: string; color2?: string; intensity?: number; duration?: number; seed?: number };

const SheetInner: React.FC<SheetProps> = (p) => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const fx = useMemo(() => {
    const x = new ShaderFx(p.kind ?? "ring", { color: p.color, color2: p.color2, intensity: p.intensity ?? 1.6, duration: p.duration, seed: p.seed });
    x.uniforms.uSheet.value = 1;
    x.mesh.scale.setScalar(2);
    return x;
  }, [p.kind, p.color, p.color2, p.intensity, p.duration, p.seed]);
  fx.set(f / fps);
  fx.mesh.visible = true;
  return <primitive object={fx.mesh} dispose={null} />;
};

export const FxSheet: React.FC = () => {
  const { width, height } = useVideoConfig();
  const p = getInputProps() as SheetProps;
  return (
    <AbsoluteFill>
      <ThreeCanvas width={width} height={height} orthographic camera={{ left: -1, right: 1, top: 1, bottom: -1, near: 0.1, far: 10, position: [0, 0, 5] }}
        gl={{ alpha: true, premultipliedAlpha: false, antialias: true }} onCreated={({ gl }) => gl.setClearColor(0x000000, 0)}>
        <SheetInner {...p} />
      </ThreeCanvas>
    </AbsoluteFill>
  );
};

// ---------------------------------------------------------------- FxLab
const KINDS: { kind: FxKind; color: string; x: number; y: number; s: number }[] = [
  { kind: "glow", color: "#7ad0ff", x: -1.6, y: 2.2, s: 1.4 },
  { kind: "energy", color: "#ff8a2a", x: 1.6, y: 2.0, s: 1.8 },
  { kind: "ring", color: "#ffd27a", x: -1.6, y: 0.0, s: 2.2 },
  { kind: "sparkle", color: "#fff0a0", x: 1.6, y: 0.0, s: 1.8 },
  { kind: "beam", color: "#ffe2a8", x: 0.0, y: 1.2, s: 5.5 },
  { kind: "dust", color: "#f3c99a", x: 0.0, y: -0.6, s: 2.6 },
];
const CUES: LightCue[] = [
  { from: 20, to: 70, spot: { at: [1.6, 0.0, 0], r: 0.22, k: 0.85 }, temp: 0.3 },
  { from: 75, to: 119, rays: { at: [0.5, 0.18], k: 0.9 }, exposure: 0.2 },
];

const LabInner: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { camera } = useThree();
  const world = useMemo(() => {
    const g = new THREE.Group();
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(20, 20), new THREE.MeshLambertMaterial({ color: "#3b4660" }));
    floor.rotation.x = -Math.PI / 2; floor.position.y = -1.5; g.add(floor);
    const wall = new THREE.Mesh(new THREE.PlaneGeometry(20, 12), new THREE.MeshLambertMaterial({ color: "#56607e" }));
    wall.position.set(0, 4, -3); g.add(wall);
    const win = new THREE.Mesh(new THREE.PlaneGeometry(2.2, 1.6), new THREE.MeshBasicMaterial({ color: new THREE.Color(3, 2.8, 2.4), toneMapped: false }));
    win.position.set(0, 4.6, -2.95); g.add(win);
    g.add(new THREE.HemisphereLight("#c9d8ff", "#3a3026", 1.2));
    const key = new THREE.DirectionalLight("#fff1dd", 1.6); key.position.set(3, 6, 5); g.add(key);
    const rail = new THREE.Mesh(new THREE.BoxGeometry(5, 0.12, 0.25), new THREE.MeshStandardMaterial({ color: "#6b4a3a", roughness: 0.5, metalness: 0.6 }));
    rail.position.set(0, -1.35, 1.2); g.add(rail);
    const fxs = KINDS.map((k, i) => { const x = new ShaderFx(k.kind, { color: k.color, intensity: 2.5, duration: 1.2, seed: i + 1, loop: true }); x.mesh.position.set(k.x, k.y, 0); x.mesh.scale.setScalar(k.s); g.add(x.mesh); return x; });
    return { g, fxs, rail };
  }, []);
  const light = useMemo(() => new LightCueEffect(), []);
  const bloom = useMemo(() => new BloomEffect({ blendFunction: BlendFunction.ADD, intensity: 0.6, luminanceThreshold: 1.0, luminanceSmoothing: 0.2, mipmapBlur: true }), []);
  const tone = useMemo(() => new ToneMappingEffect({ mode: ToneMappingMode.NEUTRAL }), []);
  world.fxs.forEach((x, i) => x.set(((f / fps) + i * 0.2) % 1.2, camera));
  heatOn(world.rail.material, cueHeat(f, 0, 110)); // trilho esquentando ao longo do lab
  applyCues(light, cueAt(CUES, f), camera);
  return (
    <>
      <primitive object={world.g} dispose={null} />
      <EffectComposer multisampling={4}>
        <primitive object={light} dispose={null} />
        <primitive object={bloom} dispose={null} />
        <primitive object={tone} dispose={null} />
      </EffectComposer>
    </>
  );
};

export const FxLab: React.FC = () => {
  const { width, height } = useVideoConfig();
  return (
    <AbsoluteFill style={{ background: "#111" }}>
      <ThreeCanvas width={width} height={height} camera={{ fov: 50, position: [0, 1, 8], near: 0.1, far: 100 }} gl={{ antialias: false }}>
        <LabInner />
      </ThreeCanvas>
    </AbsoluteFill>
  );
};
