import { continueRender, delayRender, registerRoot, staticFile } from "remotion";
import { EDL } from "./lib/edl";
import { Root } from "./Root";

// Fonte da marca: carregada antes do primeiro frame, igual em todos os renderers (edl.json > font)
if (EDL.font && typeof document !== "undefined") {
  const handle = delayRender(`fonte ${EDL.font.family}`);
  const face = new FontFace(EDL.font.family, `url('${staticFile(EDL.font.src)}')`);
  face
    .load()
    .then((f) => {
      document.fonts.add(f);
      continueRender(handle);
    })
    .catch((e) => {
      console.error("fonte não carregou", e);
      continueRender(handle);
    });
}

registerRoot(Root);
