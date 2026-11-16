import React from "react";
import { AbsoluteFill, Img, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { mix, progress } from "../lib/ease";
import type { Layer, Motion } from "../lib/edl";

// Sem movimento definido, todo still faz um push-in leve: nenhum hold fica parado.
const DEFAULT: Motion = { from: { scale: 1.0 }, to: { scale: 1.1 }, ease: "linear" };

/** Still com zoom/pan; com `layers`, faz parallax 2.5D (camadas recortadas em PNG com alpha). */
export const KenBurns: React.FC<{ src?: string; layers?: Layer[]; motion?: Motion; fit?: "cover" | "contain"; dur: number }> = ({
  src,
  layers,
  motion = DEFAULT,
  fit = "cover",
  dur,
}) => {
  const frame = useCurrentFrame();
  const { width, height } = useVideoConfig();
  const p = progress(frame, dur, motion.ease ?? "linear");
  const base = mix(p, motion.from, motion.to);
  const list: Layer[] = layers?.length ? layers : src ? [{ src, depth: 0 }] : [];
  return (
    <AbsoluteFill style={{ overflow: "hidden" }}>
      {list.map((l, k) => {
        // camadas mais próximas andam e crescem mais
        const amp = 1 + l.depth * 0.8;
        const s = 1 + (base.scale - 1) * amp;
        const tx = base.x * amp * width;
        const ty = base.y * amp * height;
        return (
          <AbsoluteFill key={k} style={{ transform: `translate(${tx}px, ${ty}px) scale(${s}) rotate(${base.rotate}deg)` }}>
            <Img src={staticFile(l.src)} style={{ width: "100%", height: "100%", objectFit: fit }} />
          </AbsoluteFill>
        );
      })}
    </AbsoluteFill>
  );
};
