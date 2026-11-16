import { Easing, interpolate } from "remotion";
import type { Ease, Pose } from "./edl";

const EASE = {
  linear: Easing.linear,
  in: Easing.in(Easing.cubic),
  out: Easing.out(Easing.cubic),
  inOut: Easing.bezier(0.45, 0, 0.55, 1), // smootherstep-like, sem "float"
};

export const EASE_FNS: Record<Ease, (t: number) => number> = EASE;

export const mix = (p: number, a: Pose = {}, b: Pose = {}): Required<Pose> => ({
  scale: (a.scale ?? 1) + ((b.scale ?? a.scale ?? 1) - (a.scale ?? 1)) * p,
  x: (a.x ?? 0) + ((b.x ?? a.x ?? 0) - (a.x ?? 0)) * p,
  y: (a.y ?? 0) + ((b.y ?? a.y ?? 0) - (a.y ?? 0)) * p,
  rotate: (a.rotate ?? 0) + ((b.rotate ?? a.rotate ?? 0) - (a.rotate ?? 0)) * p,
});

export const progress = (frame: number, dur: number, ease: Ease = "inOut") =>
  interpolate(frame, [0, Math.max(1, dur - 1)], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: EASE[ease] });
