# Cards 2D em mundo 3D (Remotion + three.js)

Leia com [cartoon 2D](cartoon-2d.md). Template local completo em `../templates/remotion-three/`, com origem/licença em `ORIGEM.md`. Motor `remotion-three`, renderer `remotion`; mygo desenha sprites sobre plates, não geometria, sombras nem pós-processamento 3D ao vivo.

## Timeline e contratos

`story.ts` é a fonte da imagem: `FPS`, `TOTAL`, `SHOTS` (id, a/b em frames, luz, multidão, `tau` = tempo da ação), `XFADE`, `FLASH_AT`, `CAMS`, `KICKS`, `dofAt` e `FOREGROUND`. Derive os frames de `beats.json` alinhado e `data/words.json`, sem reutilizar os tempos da ponte. Separe tempo do vídeo e tempo físico: um corte pode mostrar a mesma ação de outro ângulo sem reiniciar indevidamente sua fase.

`lights.ts` define `LIGHTS`/`LOOK`; `shade.ts` faz two-tone/`untone`; `paint.ts` gera texturas; `build.ts` constrói cenário e props; `world.ts` posiciona tudo por frame; `toon.ts` desenha, `cast.ts` monta o elenco e `cardshade.ts` ilumina cards. `ink.ts` exporta a layer `NOINK`; preserve o contrato com o cast. Mudou uma constante compartilhada: confira todos os leitores antes de integrar.

## Cenário, luz e material

- Mundo com frente/meio/fundo, fachadas pintadas em Canvas2D HD e detalhes que sobrevivam ao enquadramento: rodapé, cornija, peitoril, soleira, chaminé, telhado. Não cubra cada superfície com ruído fino.
- Madeira ocre com juntas escuras, pedra cinza quente/bege e ferro escuro pouco saturado. O exemplo usa casas de 7/8,4/9,8 m; são escala de referência, não dimensões obrigatórias.
- Recolora materiais dos kits CC0 pelo nome: o kit Kenney pode chegar verde-azulado. Preserve origem/licença. O template não inclui os GLBs e funciona sem eles; `kenney: true` habilita os arquivos opcionais.
- Sol relativo à câmera e congelado durante cada plano; não deixe o giro da câmera fazer a sombra saltar entre frames. Faça culling pela linha de visão para evitar lente dentro de casa ou prop bloqueando o assunto.
- Dia do exemplo: azimute 75°, elevação 48°, lit `[1.58,1.5,1.33]`, shade `[.30,.29,.36]`, lift `[.05,.06,.11]`; céu `#3e7fd2` → `#b8d0ec`, fog da cor do horizonte em 25–300 m e névoa azul distante. Frio sobretudo nas sombras.
- Golden: azimute 6°, elevação 7°, luz `#FFB36B` ×1,5, sombras violeta-azuis (~`#444380`), céu `#ffc47c` → `#8a6f8a`, fog pêssego além de 50 m e disco solar `#ffcf9e`. Evite tingir tudo de marrom/laranja.
- Cards orientados para a câmera, recebendo luz/sombra do cenário, sem emissão que os faça parecer adesivos. Sombra curta de contato sob os pés e cauda na direção do sol; rim golden somente interno na cabeça/torso, nunca cópia externa deslocada.

## Câmera e profundidade

- Grandes saltos de posição pedem corte. Para reenquadrar dentro de um plano, smootherstep de 18–40 frames; referência de pico ≤25 px/frame em 720p. Não faça fly-through enorme sem tempo/blur adequados.
- Holds com push sutil: referência 0,15–0,35%/frame, até 0,6% na palavra de impacto. São valores do estilo, sujeitos a enquadramento/QA, não zoom obrigatório para qualquer vídeo.
- Roll de 4–5° com motivo; impacto alterna sinal a cada frame e decai em ~7 frames. Sem oscilação de câmera de mão contínua.
- Foreground em aproximadamente um terço dos planos, sempre reconhecível e sem obstruir. Dissolve de 6–8 frames para tempo/lugar; flash de quatro frames apenas no momento planejado após o gancho.
- Foco nos olhos da primeira fila. Lens blur em cerca de um terço do tempo (CU/MS/inserts), não em toda a timeline; abertos e aéreos nítidos. `cInf` do exemplo a 1080 de largura: CU 12–16 px, MS 10–12, parque 7, golden 5, abertos 0. Valores de fog/DOF do estudo e do exemplo são presets diferentes, não requisitos simultâneos.

## Ink e pós

Ordem: **ink em pass próprio → lens pre → lens main → bloom → NEUTRAL tone mapping → grade**, MSAA 4. Ink junto no pass do DOF pode desaparecer. Use silhueta, salto de profundidade e dobra de normal, evitando traço no horizonte ou junção coplanar parede/chão.

Ink do exemplo `#030303`: 5,1 px perto (6 m) → 3,3 px longe (90 m), crease ×0,8 acima de 35°, salto de depth a partir de 1%, fog tint 0,3 só longe. A medição da referência sugere ~3 px a 1080 de largura: calibre por still, não misture os dois alvos. Cards vão em `NOINK`.

Bloom apenas em emissivos HDR: threshold 1, intensidade 0,5, seis níveis, raio 0,6. Escale parâmetros em pixels por altura de saída/1920 e compare still reduzido com final 2K; o clamp de DPR do R3F pode manter buffer interno em 1080 mesmo com `--scale 0.5`. Grão desligado neste look: aumentou muito o arquivo e não estava na referência. O perfil `estatico` conserva seu acabamento próprio.

## Integração e entrega

1. `novo.sh ... --motor remotion-three --formato yt-short --install` copia a demonstração técnica local. Reescreva história, layout e elenco; registre a composição em `src/Root.tsx` e confira `TOTAL`.
2. Rode `bun run --bun typecheck`, faça stills com `scripts/cartoon-stills.mjs` e siga a revisão em `skills/edicao/references/cartoon-qa.md`.
3. Renderize cada trecho ou o vídeo 3D completo para `public/clips/`. Referencie os arquivos reais como `video` no EDL (tempos/trim coerentes), preservando beats, som e legendas. Não registre caminho fictício para apenas fazer a trava passar.
4. `edl-check` avalia a cobertura do EDL; não inspeciona o movimento interno do vídeo. Stills, sequência densa e `vf qa` precisam confirmar a animação. Use `vf export` para mux/loudnorm e QA estrito por formato.

Se a câmera 3D não precisar variar depois, use `nocast` para exportar plates e aproveite os mesmos desenhos em sprites no mygo. Isso preserva o look de fundo, mas sombras/oclusões/parallax dos atores precisam ser reconstruídos no cutout.

## Contato e referencial em cenas compostas

Use o [QA visual em cinco etapas](../../edicao/references/qa-cinco-etapas.md) ao integrar
modelos e billboards. Exporte âncoras semânticas de móveis (assento, encosto, apoio dos pés)
e derive a posição do quadril da pose, não do centro da textura. Valide limites durante
a ação e a rotação para a câmera. Para transições de superfície como oceanos e continentes,
prefira uma máscara/estado contínuo no mesmo globo a cascas opacas concorrentes.
