import React from "react";
import { Composition } from "remotion";
import { BridgeShort, BRIDGE_FRAMES } from "./videos/cartoon/Bridge";
import { FPS } from "./videos/cartoon/story";
import { FxSheet, FxLab } from "./fx/FxSheet";
// Demonstração técnica herdada da ponte. Substitua história, elenco e layout antes de produzir outro vídeo.
export const Root: React.FC = () => (
  <>
    <Composition id="CartoonShort" component={BridgeShort} durationInFrames={BRIDGE_FRAMES} fps={FPS} width={1080} height={1920} />
    {/* FX por shader: FxSheet vira pack de sprites (fx-pack.mjs); FxLab é a folha de conferência de FX e luz */}
    <Composition id="FxSheet" component={FxSheet} durationInFrames={30} fps={30} width={512} height={512} />
    <Composition id="FxLab" component={FxLab} durationInFrames={120} fps={30} width={1080} height={1920} />
  </>
);
