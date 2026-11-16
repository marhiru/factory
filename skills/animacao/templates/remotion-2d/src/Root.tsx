import React from "react";
import { Composition } from "remotion";
import { Main } from "./Main";
import { FPS, TOTAL_FRAMES } from "./lib/edl";
import { FORMATS } from "./lib/formats";

// Uma composição por formato de entrega; o mesmo edl.json alimenta todas.
export const Root: React.FC = () => (
  <>
    {Object.values(FORMATS).map((f) => (
      <Composition key={f.id} id={f.id} component={Main} defaultProps={{ format: f.id }} durationInFrames={TOTAL_FRAMES} fps={FPS} width={f.width} height={f.height} />
    ))}
  </>
);
