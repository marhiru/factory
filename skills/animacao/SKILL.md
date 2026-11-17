---
name: animacao
description: "Animação da fábrica: cutout 2D com sprites e plates no mygo/Remotion, desenho e atuação cartoon Canvas2D, personagens 2D iluminados em mundo 3D com three.js (template incluído), câmera, ink, DOF e pós; também motion graphics, vídeo IA, mapas 3D e motor estático para yt-long. Use para animar personagens/cenas ou preparar cenas e EDL de um projeto video-factory. Inclui edl-check e checagem visual; Ken Burns não conta como animação no perfil animado."
---

# Animação (etapa 3 da fábrica)

Ferramental JS/TS: siga [Bun e tsgo/tsrs](../factory/references/ferramental.md) ao executar ou alterar scripts e templates.

Entrada: `roteiro/beats.json` com tempos (`vf align`), `data/words.json`, `assets/style-bible.md`. Saída: `data/edl.json` com **cenas animadas**, aprovado pelo `edl-check`, e stills de checagem conferidos.

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf` · `VFM=${CLAUDE_PLUGIN_ROOT}/bin/vf-mygo` · `B2E=${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/beats2edl.mjs` · `CHK=${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/edl-check.mjs`

## Cartoon 2D e personagens 2D em mundo 3D
- Desenho Canvas2D, proporções, expressões, piscadas, atuação e exportação: [cartoon 2D](references/cartoon-2d.md).
- Timeline, mundo, cards iluminados, câmera, ink, DOF e pós: [2D em mundo 3D](references/2d-in-3d-world.md).
- Template executável local: `templates/remotion-three/`; origem MIT em `ORIGEM.md`. Use `toon.ts` para sprites mesmo quando o renderer final for mygo.
- **Luz de cenário, FX e shaders:** [luz-fx-shaders.md](references/luz-fx-shaders.md). Rig por cenário (interno e global) para harmonia, cues dinâmicos (`spot`, `rays`, exposição, temperatura) para destacar personagem e exaltar a cena, régua de qualidade de FX e o motor GLSL em `templates/remotion-three/src/fx/` (`ShaderFx`, `LightCueEffect`, composições `FxSheet`/`FxLab`).
- FX em sprites para o mygo: `bun scripts/fx-pack.mjs <efeito-pixelfx | sheet.json | pasta de PNGs> public/sprites/fx-<nome>` (pixelfx e pixel-splats ficam em `ref/fx/` depois do `vf init`).
- Neste perfil, atuação pontual e câmera podem sustentar o plano; não imponha idle em figurantes. O movimento precisa existir na imagem, e não só nas flags do EDL.

## Regra número 1: animar é obrigatório
Stills com zoom e pan (Ken Burns) e cartelas de texto **não são animação**. Num vídeo animado, alguém ou algo **faz** alguma coisa em cada plano: entra, anda, marcha, acena, muda de expressão na palavra certa, um prop cai, um balão aparece.

- Ken Burns é só respiro: no máximo 30% do tempo, e nunca mais de 6 s seguidos.
- **Trava:** `bun $CHK data/edl.json` precisa sair com código 0 (pelo menos 70% do tempo com animação real, e nenhum trecho sem animação acima de 6 s) antes de marcar `etapas.animacao: feito`. Se reprovar, a etapa continua `em_andamento`, sem exceção.
- Erro registrado: num projeto anterior (short de história), a fábrica entregou só stills com Ken Burns e marcou `animacao: feito`. Foi por isso que essa trava foi criada.

## Contrato: `data/edl.json`
Uma timeline em segundos com clipes `scene | image | video | text | color`, faixas de áudio e legendas. A especificação completa, incluindo `scene` e o `sprite.json`, está em `templates/edl.schema.md`. Quem renderiza é o `renderer` do `factory.yaml`.

## Pipeline cutout 2D (padrão dos motores `cutout-2d` e `remotion-2d`)
1. **Elenco (sprites).** Cada personagem e prop vira um *pack* em `public/sprites/<nome>/`: PNGs com transparência e um `sprite.json`. Os estados são as expressões (cada uma com o frame de piscada) e os ciclos (marcha, passeio, aceno, dança e efeitos de 8 frames).
   - Se o projeto desenha personagens em código (Canvas2D), use o exportador `templates/sprites/`. Registre os desenhos em `sprites/registry.ts` (tipo `Pack` de `types.ts`) e rode:
     `REMOTION_BROWSER=<chrome-headless-shell> bun sprites/export-sprites.mjs sprites/index.ts public/sprites`
     Referência de tempo: 371 PNGs em ~4 min.
   - Se os personagens vêm de imagem (ilustração, IA, banco), recorte em PNG com alpha, um arquivo por expressão e por frame de ciclo, e escreva o `sprite.json` à mão.
   - O mínimo por personagem principal: 3 expressões com piscada e 1 ciclo. Coadjuvante: neutro, reação e um ciclo de comemoração.
2. **Cenários (plates).** Um fundo por plano, **sem personagens**, em `public/img/plates/`.
   - Se existe um mundo 3D (Remotion + three.js), renderize o mesmo quadro sem o elenco: um interruptor `nocast` no `stage()` e um still por plano (~1,8 s cada, ver `templates/remotion-three.md`).
   - Se não existe, use ilustração, foto CC0 ou uma pintura gerada, no formato vertical ou horizontal certo.
3. **Cenas.** Um clipe `scene` por plano: plate, `camera` (push ou pan, com parallax pela `depth`) e `actors`.
   - Para cada ator: posição (`x`, `y` = ponto dos pés, em fração do quadro), altura `h`, `state` inicial, `keys` (movimento e troca de estado), `enter`/`exit` (`pop`, `slide-left/right`, `drop`, `rise`, `fade`), `idle` (`breathe`, `bob`, `sway`, `float`, `jitter`), `shadow` e `clip_y` (atrás de mesa ou muro).
   - **Toda ação cai numa palavra:** `at_word: N` (o motor adianta 2 frames) ou o tempo calculado de `data/words.json`.
   - Escreva as cenas como script com a biblioteca `vfscene.py` (o `novo.sh` copia para `roteiro/`, com o exemplo em `roteiro/cenas.py`: `init()`, `w(i)`, `find("palavra")`, `scene()`, `A()`, `fx()`, `st()`, `push()`, `pan()`, `save()`), não como JSON à mão: refazer depois de mudar a voz vira um comando.
   - O `beats2edl.mjs` gera só um **animatic** de stills, útil para cronometrar. Ele não substitui as cenas.
4. **Trava:** `bun $CHK data/edl.json` precisa sair com código 0.
5. **Checagem visual** (abaixo) e render.

### Reaproveitar um projeto 3D ou animado que já existe
Quando já existe um vídeo (Remotion + three.js, por exemplo) e a ideia é refazer ou derivar em cutout 2D:
1. **Planos do original:** `$VF frames <video_original.mp4> -o ref/orig` gera `shots.json` com os cortes. Uma cena por plano mantém o ritmo que já funcionava.
2. **Plates sem elenco:** adicione uma inputProp (`nocast`) que esconde o elenco no `stage()`/world do projeto e rode `templates/plates/plates.mjs --composition <Id> --frames <frame:nome,...> --scale 1.3333` (~1,8 s por plate).
3. **Sprites dos mesmos desenhos:** importe as funções de desenho do projeto no `sprites/registry.ts` e use os construtores de `templates/sprites/packs.ts` (`front`, `side`, `fx`, `marchFrames`, `strollFrames`, `cheerFrames`, `danceFrames`).
4. **Cenas:** `roteiro/cenas.py` com `vfscene.py`, posicionando os atores onde estavam no original (stills de `ref/orig` lado a lado com `-still`).

Lições do primeiro projeto feito assim (short de 110 s, 33 cenas):
- O QA pegou duas cenas sem vida (personagem sozinho sobre fundo desfocado, cenário escuro). Fundo liso ou desfocado pede mais câmera e uma ação a mais.
- Um personagem sozinho num plano > 3 s precisa de pelo menos 2 ações (troca de expressão + gesto ou deslocamento).
- Gerar a cena num script (`vfscene.py`) permitiu reajustar 33 cenas em segundos quando os tempos da voz mudaram.

### FX de exagero (o que faz o short "explodir")
Catálogo com receitas em `skills/analise/referencias/rickclips.md`, estudado num short de 6,5M views com o mesmo estilo de bonecos 2D em mundo 3D:
- **Um FX de exagero por ato:**
  - impacto com *hold* no ar: `rot` sobe em 0,15 s, segura ~0,8 s e cai;
  - fúria/hipérbole no clímax: olhos vermelhos, chão rachando, onda de choque com flash;
  - **mudança de clima** na explicação: noite, chuva, POV.
- **Transição de assinatura** na passagem história → explicação: zoom-through no olho (clipe de 0,3 s com câmera `scale` 1,2 → 6 no olho) ou whip.
- **Props-piada que contam a história:** placa-contador que vira de 7 para 0 na batida, letreiro com lâmpadas piscando (ciclo de 2 frames).
- **Termo do fenômeno dentro do mundo:** placa de rodovia ou letreiro, e não cartela.
- **Plano longo só com câmera trabalhando:** vários enquadramentos do mesmo cenário costurados por dissolves de 0,2-0,3 s (`fade` 0,2-0,3) com a câmera seguindo no mesmo sentido através do dissolve (medido: 75-100 % das transições das referências).

### Direção de animação (o que faz parecer vivo)
- Personagem parado sempre respira (`idle: breathe`) e pisca sozinho (automático, 0,5-2,3 s). Na troca de expressão o motor fecha os olhos para esconder o corte.
- **Reação antes da fala:** a expressão muda 0-3 frames antes da palavra (o `at_word` já faz isso).
- Quem fala do lugar onde está: grupo reagindo em cascata, com 0,06-0,25 s entre cada um.
- Prop entra com física: `drop` com quique, `pop` com mola. Impacto ganha um efeito de 0,3 s (burst).
- Andar ou marchar = estado em ciclo + `keys` movendo `x`/`y`. Aproximar da câmera = `y` e `scale` crescendo juntos.
- Balões de pensamento e boato entram em `pop` na palavra e flutuam (`idle: float`).
- A câmera nunca para: push de 4-15% por plano, ou pan com `scale ≥ 1 + 2|x|`. Plano longo (> 5 s) ou fundo liso pede mais câmera e mais ação.
- **Trocar de assunto no mesmo lugar:** chicote (pan rápido de 4-6 frames com `scale` subindo), atravessar porta/janela (push até a moldura encher o quadro), rack focus no 3D, ou FX `dust` como cortina. Corte seco ≤ 25 % das transições.
- **Trechos contínuos com contexto semelhante:** mesmo cenário, sem corte. A câmera faz zoom-out do enquadramento atual e zoom-in no novo ponto de interesse numa `scene` só: `cam=junta(s_a, foco_a, s_b, foco_b)` do `vfscene.py` (câmera `from → via → to`, mygo e Remotion). O `edl-check` reprova corte seco entre clipes vizinhos com o mesmo plate. Corte ou transição só quando muda lugar, tempo ou assunto.

## Motor `estatico` (yt-long com tema)
Exceção à regra 1, só para vídeo longo: imagens curadas + `roteiro/cenas.py` (cópia de `templates/cenas/estatico.py`) → `data/edl.json` com `perfil: estatico` → `edl-check` no perfil estático → HyperFrames. Regras, cards e técnicas de edição em `templates/estatico.md`.

## Renderer: `mygo`, `hyperframes` ou `remotion`
| | `mygo` (`bin/vf-mygo`) | `hyperframes` (`scripts/edl2hf.mjs`) | `remotion` (template `remotion-2d/`) |
|---|---|---|---|
| Como | Go nativo, GPU (Metal), encoder de hardware, sem navegador | HTML + GSAP compilado do edl.json, Chrome headless com workers | Chrome headless, um screenshot por frame |
| Velocidade medida (M4) | short animado (110 s, 33 cenas, 141 atores): **30 s em 2K**, 19 s em 1080p | 60 s de 1080p em 39 s (12 min ≈ 8 min) | ~4x mais lento que o mygo |
| Cobre | `scene` (cutout completo), image, video, text, color, transições, legendas | image (zoom/pan/camadas), video, text, color, **`card`** (capítulo, dado, citação, mapa, comparação), transições (fade, blur, dip, flash, slide, punch), **overlays** (grão, vinheta, light leak, graduação) | tudo do mygo + componentes React, three.js, Studio |
| Padrão para | shorts/TikTok (cutout) | vídeo longo (`estatico`; `kenburns`/`ia-video` em yt-long) | componente sob medida |

- mygo: `$VFM -project . -formato <f> -o out/video_<f>.mp4` (`-scale 1.3333` para 2K, `-still N -o f.png` para checagem). Sai sem áudio: o som entra no `vf export --audio`.
- hyperframes: `${CLAUDE_PLUGIN_ROOT}/skills/edicao/scripts/render-hf.sh <formato> out/video_<f>.mp4` (compila, faz lint e renderiza; `--draft`, `--from/--to`). Sai sem áudio. Paridade com o mygo em clipes de Ken Burns: SSIM ≥ 0,99. Recusa clipes `scene`.
- A matemática de animação é a mesma nos três (`tools/vf-mygo/scene.go` = `src/lib/anim.ts`). Tipo de clipe novo precisa ser implementado nos dois, ou o projeto fica só no Remotion.

## Outros motores
- **`ia-video`:** gere `data/shots.yaml` com `templates/ia-video.md`, produza os clipes (image-to-video a partir do plate e do elenco, para manter consistência) e use clipes `video`. Clipe com defeito (mãos, texto, morph) é cortado, nunca entregue. Clipes de IA contam como animação no `edl-check`.
- **`remotion-three`:** use o template local `templates/remotion-three/` e [2D em mundo 3D](references/2d-in-3d-world.md). O `beats.json` alinhado fornece planos e reações; confira a animação no clipe renderizado antes de registrá-lo como `video` no EDL.
- **`mapas-3d`:** delegue à skill **`mapas-3d-videos-virais`**.
- **Motion graphics** (gráfico, mapa 2D, diagrama): componente em `src/components/` + tipo de clipe novo. Cartela parada não conta como animação, o elemento precisa se construir ou se mover.
- **`kenburns`:** só para trechos de respiro dentro de um vídeo animado, ou quando o usuário pedir explicitamente um vídeo de fotos (documentário de arquivo). Nesse caso, registre o pedido em `factory.yaml > notas` e rode o `edl-check` com `--min 0`.

## Luz e FX (passes 3 e 4 da esteira)
- Iluminador e artista de FX são agentes próprios (esteira, seção 3b). Nenhum plano fica com luz chapada sem fonte, e nenhum FX entra sem antecipação → pico → dissipação e luz no entorno.
- Ordem no pós do `remotion-three`: ink → DOF → **LightCue** → bloom → tone mapping → grade. Os FX `ShaderFx` emitem acima de 1 para florescer.
- Confira no `FxLab` (stills em 3 frames) antes de usar um shader novo; no mygo, confira o pack de FX sobre o plate claro e o escuro do vídeo.

## Checagem visual (sempre, antes da edição)
1. Um still por cena, no momento da ação principal (`$VFM ... -still N`, ~3 s por still com sprites), juntado numa folha. Confira posição dos pés no chão do plate, escala coerente com a perspectiva, quem fica atrás de quê (`clip_y`), legibilidade no celular e safe zone.
2. Render completo com o mygo e `$VF qa`. A folha densa (`out/qa_*/sheets`) mostra a animação quadro a quadro: confirme que as ações acontecem nas palavras certas.
3. Congelamento no QA = cena sem vida suficiente. Aumente a câmera ou dê uma ação a alguém, não esconda o aviso.
4. Só então marque `etapas.animacao: feito`, com o resultado do `edl-check` em `notas`.

## Regras de movimento (padrão da fábrica)
- Nenhum hold estático de 12 frames ou mais (o `vf qa` acusa).
- Corte seco é o padrão. Fade (0,25-0,3 s) só em mudança de tempo ou lugar. Flash branco só logo depois do gancho.
- O visual reage 0-3 frames **antes** da palavra.
- Plano médio nos shorts: 1,5-3 s. No longo: 3-6 s, até 8 s com movimento forte.
- Texto na tela: no máximo 5 palavras, por no mínimo 1 s, e dentro da safe zone.

## Contato, coesão e câmera

Antes de aceitar cenas compostas, siga as etapas 2, 3 e 5 do [QA visual em cinco etapas](../edicao/references/qa-cinco-etapas.md).
Defina contatos de assento/colo/mãos/pés e suportes dos props; confira início, extremos e fim
em vídeo. Billboards sentados não podem girar para dentro do móvel. Efeitos usam o referencial
e a fase do objeto. Reveja margens e oclusões depois de qualquer mudança de pose ou câmera.
Asset mal posicionado, que quebre a harmonia do cenário ou tape a cena reprova. Rosto e ação principal ficam fora da faixa da legenda (0,66-0,76 da altura). Props e luz do cenário ficam no mesmo lugar em todos os ângulos.
Identidade: com o elenco aprovado, `bun scripts/elenco-lock.mjs snapshot`; toda correção roda `elenco-lock.mjs check --pedido <posicao|pose|novo-estado>` (código 0). Emoção e passagem de tempo são estados do mesmo pack. Correção pedida
pelo usuário mexe só no layout/pose do mesmo asset (mesmo pack), nunca o troca por outro
parecido (etapa 3b).
O `edl-check` mede cobertura de animação e não certifica essas propriedades.
