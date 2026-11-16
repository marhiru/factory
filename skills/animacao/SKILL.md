---
name: animacao
description: "Templates de animação da fábrica de vídeo: Remotion 2D orientado a data/edl.json (motion graphics, texto cinético, legendas karaokê), Ken Burns/parallax para stills, planos de geradores de vídeo IA (Veo, Kling, Runway…) e ponte para Remotion+three.js (cartoon-3d-short) e mapas 3D. Use na etapa de animação de um projeto video-factory ou para montar a timeline (edl.json) de um vídeo."
---

# Animação (etapa 3 da fábrica)

Entrada: `roteiro/beats.json` com tempos (`vf align`), `data/words.json`, assets em `public/`. Saída: `data/edl.json` revisado e stills de checagem aprovados.

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf` · `VFM=${CLAUDE_PLUGIN_ROOT}/bin/vf-mygo` · `B2E=${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/beats2edl.mjs`

## Contrato: `data/edl.json`
Todos os motores falam a mesma língua: uma timeline em segundos com clipes `image | video | text | color`, faixas de áudio e legendas. Especificação em `templates/edl.schema.md`. Quem renderiza é o `renderer` do `factory.yaml` (hoje Remotion, com o template `templates/remotion-2d/` já copiado para o projeto pelo `novo.sh`).

## Renderer: `remotion` ou `mygo`
`factory.yaml > renderer` escolhe quem desenha o `edl.json`. Os dois leem o mesmo arquivo e as mesmas fontes.

| | `mygo` (`bin/vf-mygo`) | `remotion` (template `remotion-2d/`) |
|---|---|---|
| Como | Go nativo, frames desenhados na GPU (Metal) e encoder de hardware, sem navegador | Chrome headless, um screenshot por frame |
| Velocidade medida (M4) | 22 s vertical 1080p: **3,4 s** (198 fps, 0,9 s de CPU); em 2K: 6,6 s | 16,7 s (53,6 s de CPU); em 2K: 23,4 s |
| Clipes de vídeo | 150/150 frames exatos (ffmpeg por clipe) | 149/150 |
| Cobre | image (Ken Burns, parallax), video, text (title/lower/label/number), color, transições, legendas | tudo isso + qualquer componente React, three.js, Remotion Studio |
| Use para | **produção em escala de `remotion-2d` e `kenburns`/`ia-video`** | componentes sob medida, preview interativo, 3D (`cartoon-3d-short`) |

- mygo: `$VFM -project . -formato <f> -o out/video_<f>.mp4` (`-scale 1.3333` para 2K, `-still N -o f.png` para checagem, `-bitrate`). Ele sai sem áudio, como o Remotion com `--muted`: o som entra no `vf export --audio`.
- Os dois renderers dão a mesma imagem (SSIM 0,98 no teste, com as fotos a ~41 dB de PSNR). A diferença está na quebra de linha de legendas que ficam no limite da largura, porque os motores de texto medem alguns pixels diferente. Não troque de renderer no meio de um projeto já aprovado.
- Tipo de clipe novo: no Remotion é um componente React. No mygo, implemente também em `tools/vf-mygo/main.go` (função `clip`), ou use o Remotion só nesse projeto.

## Por motor
### `remotion-2d` / `kenburns`
1. Primeiro corte automático: `node $B2E roteiro/beats.json data/edl.json --voice voice/voice.wav` (`--music audio/music.wav` se houver).
2. Refine o `edl.json` à mão: motion por plano, `texto_tela`, transições e camadas de parallax (`layers` com PNGs recortados, `depth` 0 = fundo, 1 = frente).
3. Para animação que não cabe no EDL (gráfico animado, mapa, diagrama), crie um componente em `src/components/` e um tipo de clipe novo em `ClipView.tsx`. Mantenha o tempo amarrado a palavras: `wordAt(k)` em `src/lib/edl.ts`.
4. Guia de movimento em `templates/kenburns.md`.

### `ia-video`
1. Gere `data/shots.yaml` a partir dos beats com `templates/ia-video.md` (um prompt por plano, com bloco de consistência fixo).
2. Gere os clipes no provedor escolhido (o usuário ou a API dele). Salve em `public/clips/bNN-k.mp4` e registre no manifest com `origem: ia-video`, provedor, modelo e seed.
3. Liste os clipes em `assets` de cada beat e rode `beats2edl.mjs`. Os clipes `video` usam `trim` e `playbackRate` para casar com a fala.
4. Clipes de IA com defeito (mãos, texto ilegível, morph) são cortados ou cobertos por um insert, nunca entregues.

### `remotion-three`
Delegue à skill **`cartoon-3d-short`** (story.ts, world, cast, ink e pós). O `beats.json` com tempos substitui a etapa de cronometragem manual.

### `mapas-3d`
Delegue à skill **`mapas-3d-videos-virais`**.

## Checagem visual (sempre, antes da edição)
1. Stills nos momentos-chave (gancho, cada virada, payoff, último frame):
   `npx remotion still src/index.ts <formato> out/stills/f<frame>.jpg --frame=<frame> --scale=0.5` ou, com mygo, `$VFM -project . -formato <formato> -still <frame> -o out/stills/f<frame>.png` (~0,6 s)
2. Abra os stills e confira: legível no celular, nada importante fora da safe zone, sem hold estático, o visual bate com a fala daquele instante.
3. Render de prévia rápido: `$VFM -project . -formato <formato> -o out/preview.mp4` (o render completo do mygo já é rápido o bastante para prévia) ou `npx remotion render src/index.ts <formato> out/preview.mp4 --scale=0.5 --crf=28`. Depois rode `$VF keyframes out/preview.mp4 -o out/preview_kf` e olhe as sheets: cada plano deve ser distinto, e um corte sobrando denuncia clipe duplicado.
4. Marque `etapas.animacao: feito`.

## Regras de movimento (padrão da fábrica)
- Nenhum hold estático ≥ 12 frames: todo still tem push-in ou pan leve. O `vf qa` acusa congelamento.
- Corte seco é o padrão. Fade (0,25-0,3 s) só em mudança de tempo ou lugar. Flash branco só logo depois do gancho.
- O visual reage 0-3 frames **antes** da palavra (o `beats2edl` já adianta 2 frames).
- Plano médio nos shorts: 1,5-3 s. No longo: 3-6 s, até 8 s com movimento forte.
- Texto na tela: no máximo 5 palavras, por no mínimo 1 s, e dentro da safe zone (`src/lib/formats.ts`).
