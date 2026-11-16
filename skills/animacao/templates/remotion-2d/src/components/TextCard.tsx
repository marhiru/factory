import React from "react";
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import type { Safe } from "../lib/formats";
import { FONT } from "../lib/edl";

const stroke = (px: number): React.CSSProperties => ({ WebkitTextStroke: `${px}px #000`, paintOrder: "stroke fill" });

export const TextCard: React.FC<{ text: string; style?: "title" | "lower" | "label" | "number"; color?: string; dur: number; safe: Safe }> = ({
  text,
  style = "title",
  color = "#fff",
  dur,
  safe,
}) => {
  const frame = useCurrentFrame();
  const { fps, width, height } = useVideoConfig();
  const u = Math.min(width, height) / 1080; // escala de px
  const pop = spring({ frame, fps, config: { damping: 14, stiffness: 180, mass: 0.6 } });
  const out = interpolate(frame, [dur - 6, dur], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  if (style === "number") {
    // "1.889" e "12.000" têm separador de milhar: tira antes de converter ("3,5" vira 3.5)
    let raw = text.match(/-?[\d.,]+/)?.[0] ?? "0";
    if (/\d[.,]\d{3}(?!\d)/.test(raw)) raw = raw.replace(/[.,](?=\d{3}(?!\d))/g, "");
    const target = Number(raw.replace(",", ".")) || 0;
    const v = interpolate(frame, [0, Math.min(dur, fps * 1.2)], [0, target], { extrapolateRight: "clamp" });
    // mantém o estilo do original: "1889" (ano) sem separador, "12.000" com separador
    const grouped = /\d[.,]\d{3}/.test(text);
    const n = Math.round(v);
    const shown = text.replace(/[\d.,-]+/, grouped ? n.toLocaleString("pt-BR") : String(n));
    return (
      <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: out }}>
        <div style={{ fontFamily: FONT, fontWeight: 900, fontSize: 190 * u, color, ...stroke(10 * u), transform: `scale(${0.6 + 0.4 * pop})` }}>{shown}</div>
      </AbsoluteFill>
    );
  }
  if (style === "lower") {
    const x = interpolate(pop, [0, 1], [-width * 0.6, 0]);
    return (
      <AbsoluteFill style={{ justifyContent: "flex-end", padding: `0 ${safe.right + 60 * u}px ${safe.bottom + 140 * u}px ${safe.left + 60 * u}px`, opacity: out }}>
        <div style={{ transform: `translateX(${x}px)`, background: "rgba(0,0,0,.72)", color, fontFamily: FONT, fontWeight: 800, fontSize: 54 * u, padding: `${18 * u}px ${30 * u}px`, borderLeft: `${10 * u}px solid #ffd23f`, alignSelf: "flex-start" }}>
          {text}
        </div>
      </AbsoluteFill>
    );
  }
  if (style === "label") {
    return (
      <AbsoluteFill style={{ padding: `${safe.top + 60 * u}px ${safe.right + 60 * u}px 0 ${safe.left + 60 * u}px`, opacity: out }}>
        <div style={{ alignSelf: "flex-start", background: "#ffd23f", color: "#111", fontFamily: FONT, fontWeight: 900, fontSize: 44 * u, padding: `${10 * u}px ${22 * u}px`, borderRadius: 10 * u, transform: `scale(${pop})` }}>{text}</div>
      </AbsoluteFill>
    );
  }
  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: `${safe.top}px ${safe.right + 60 * u}px ${safe.bottom}px ${safe.left + 60 * u}px`, opacity: out }}>
      <div style={{ fontFamily: FONT, fontWeight: 900, fontSize: 120 * u, lineHeight: 1.05, textAlign: "center", color, textTransform: "uppercase", ...stroke(8 * u), transform: `scale(${0.7 + 0.3 * pop}) translateY(${(1 - pop) * 40 * u}px)` }}>
        {text}
      </div>
    </AbsoluteFill>
  );
};
