# Exportador de sprites (cutout 2D)

Transforma qualquer desenho Canvas2D em packs de sprites (`public/sprites/<pack>/*.png` + `sprite.json`) para os clipes `scene`.

1. Copie esta pasta para `<projeto>/sprites/` (o `novo.sh` já faz isso nos motores `cutout-2d` e `remotion-2d`).
2. Escreva `registry.ts`: um `Pack` por personagem ou prop. Cada `SpriteItem` desenha um PNG: um frame de um estado, ou o frame `blink` desse estado.
3. Rode (precisa de `remotion`, `@remotion/bundler` e `@remotion/renderer` instalados):
   ```
   REMOTION_BROWSER=<caminho do chrome-headless-shell> node sprites/export-sprites.mjs sprites/index.ts public/sprites [pack1,pack2]
   ```
   `REMOTION_BROWSER` reaproveita o navegador que o CLI do Remotion já baixou (`node_modules/.remotion/chrome-headless-shell/...`). Sem ele, o Remotion baixa um.

Regras do elenco: personagem principal com pelo menos 3 expressões (cada uma com piscada) e 1 ciclo. Coadjuvante com neutro, reação e comemoração. Efeitos (balão, impacto, brilho) em 8 frames.
