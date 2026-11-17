import React, { useMemo } from "react";
import { AbsoluteFill, Img, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { actorAt, prepare, SPRITES } from "../lib/anim";
import type { Clip, SceneSpec } from "../lib/edl";
import { EASE_FNS, mix, progress } from "../lib/ease";
import type { Motion } from "../lib/edl";

/** espelho do camAt do vf-mygo: com `via`, from -> via (zoom-out) -> to (zoom-in), cada perna com o ease (padrão inOut) */
const camAt = (m: Motion, frame: number, dur: number) => {
  if (!m.via) return mix(progress(frame, dur, m.ease ?? "linear"), m.from, m.to);
  const e = EASE_FNS[m.ease ?? "inOut"] ?? EASE_FNS.inOut;
  const u = Math.min(1, Math.max(0, frame / Math.max(1, dur - 1)));
  const at = Math.min(0.95, Math.max(0.05, m.via_at ?? 0.5));
  return u < at ? mix(e(u / at), m.from, m.via) : mix(e((u - at) / (1 - at)), m.via, m.to);
};

/** Cena cutout 2D: plate + câmera com parallax + atores-sprite animados (espelho do scene.go do vf-mygo). */
export const Scene: React.FC<{ clip: Clip & { type: "scene"; scene: SceneSpec }; dur: number }> = ({ clip, dur }) => {
  const frame = useCurrentFrame();
  const { fps, width, height } = useVideoConfig();
  const sc = clip.scene;
  const t = frame / fps;
  const actors = useMemo(() => sc.actors.map((a) => prepare(clip, a)), [clip, sc.actors]);
  const m = sc.camera;
  const cam = m ? camAt(m, frame, dur) : { scale: 1, x: 0, y: 0, rotate: 0 };
  const withCam = (depth: number, body: React.ReactNode, key: string) => {
    const amp = 1 + depth * 0.5;
    const s = 1 + (cam.scale - 1) * amp;
    return (
      <AbsoluteFill key={key} style={{ transform: `translate(${cam.x * amp * width}px, ${cam.y * amp * height}px) scale(${s}) rotate(${cam.rotate}deg)` }}>
        {body}
      </AbsoluteFill>
    );
  };
  return (
    <AbsoluteFill style={{ overflow: "hidden" }}>
      {sc.bg &&
        withCam(
          0,
          <>
            {sc.bg.color && <AbsoluteFill style={{ background: sc.bg.color }} />}
            {sc.bg.src && <Img src={staticFile(sc.bg.src)} style={{ width: "100%", height: "100%", objectFit: sc.bg.fit ?? "cover" }} />}
          </>,
          "bg",
        )}
      {actors.map((a) => {
        const sp = SPRITES[a.sprite];
        if (!sp) return null;
        const p = actorAt(a, sp, t, frame);
        if (!p.visible || p.op <= 0.001) return null;
        const ph = (a.h ?? 0.5) * height, pw = ph * (sp.w / sp.h);
        const [ax, ay] = sp.anchor ?? [0.5, 0.975];
        const sw = pw * 0.42 * Math.abs(p.sx), shh = sw * 0.22;
        const body = (
          <>
            {a.shadow && (
              <div style={{ position: "absolute", left: p.x * width - sw / 2, top: p.y * height - shh / 2, width: sw, height: shh, borderRadius: "50%", background: `rgba(36,23,58,${0.32 * p.op})`, filter: "blur(4px)" }} />
            )}
            <Img
              src={staticFile(p.img)}
              style={{
                position: "absolute", left: p.x * width - ax * pw, top: p.y * height - ay * ph, width: pw, height: ph, opacity: p.op,
                transformOrigin: `${ax * 100}% ${ay * 100}%`, transform: `scale(${(p.flip ? -1 : 1) * p.sx}, ${p.sy}) rotate(${p.rot}deg)`,
              }}
            />
          </>
        );
        return withCam(
          a.depth ?? 0,
          a.clip_y ? <div style={{ position: "absolute", left: 0, top: 0, width, height: a.clip_y * height, overflow: "hidden" }}>{body}</div> : body,
          a.id,
        );
      })}
    </AbsoluteFill>
  );
};
