# Mundo 3D de alta qualidade (Blender) com personagens cutout em cards

> **Escopo: só shorts (`yt-short` e `tiktok`).** O motor `blender-3d` nunca é usado em `yt-long`: o vídeo longo da fábrica é o motor `estatico` (imagens + HyperFrames). Motivo: custo de render e de produção de cena por minuto; no short, o mundo 3D é o diferencial que segura o polegar.

Objetivo: o nível de cenário do estilo estudado em `skills/analise/referencias/rickclips.md`: mundo PBR estilizado com luz de cinema, câmera sempre em movimento, personagens 2D de traço preto em cards dentro do 3D. Status: **especificação**. O motor `blender-3d` ainda não foi implementado nem testado na fábrica (o Blender não estava instalado quando isto foi escrito).

## Por que Blender
- Gratuito, roda **headless** (`blender -b arquivo.blend -P script.py -- args`) e é 100% scriptável em Python (`bpy`): o Claude monta a cena por código, como já fazemos com three.js.
- **Eevee** (tempo quase real, ~0,3-2 s/frame a 1080x1920 num M4 dependendo de volumétricos) para produção diária; **Cycles** só para planos-herói.
- Tem tudo o que o estilo usa: Geometry Nodes (espalhar grama/pedras), Cell Fracture (quebrar o chão), partículas, volumétricos, motion blur, bloom, AgX (tone mapping suave = "cores suaves").
- Alternativa equivalente: Unreal Engine 5 (Lumen + Megascans), mais pesado de automatizar.

## Peças do pipeline
| Peça | Como | Reaproveita |
|---|---|---|
| **Kit de mundo** (por canal) | `.blend` de biblioteca: terreno, estrada, cerca, árvores, grama (Geometry Nodes scatter), céu/HDRI, nuvens volumétricas, materiais PBR | assets CC0: Poly Haven (HDRI, texturas, modelos), ambientCG (texturas), Quaternius e Kenney (modelos estilizados) |
| **Set por cenário** | `world/<cenario>.json` → script `bpy` monta o set a partir do kit (posições de props, hora do dia, clima) | `cenario` do `beats.json` |
| **Elenco** | PNGs do exportador de sprites (`templates/sprites/`) como **image planes** com alpha (`Import Images as Planes`), material emissivo + sombra ligada, virados para a câmera | os mesmos packs do cutout (estados, piscada, ciclos) |
| **Animação dos atores** | as `keys`/`state`/`at_word` do clipe `scene` (mesmo `edl.json`). Uma etapa amostra a matemática do `anim.ts` por frame e grava `x,y,scale,rot,state,frame` → o `bpy` aplica nos cards | `scene.go`/`anim.ts` (fonte única da animação) |
| **Câmera** | curva (Bezier) por plano + `Follow Path`, `ease`, shake por noise modifier; DOF no foco do ator principal | `camera` do clipe, estendida com `path` |
| **FX** | partículas (poeira, chuva, faíscas), Cell Fracture (chão quebrando), anel de choque (malha + shader), volumétrico (god rays), luzes emissivas piscando (letreiro) | catálogo de FX (`glossario-fx.md`) |
| **Render** | Eevee, 1080x1920 (ou 1440x2560), 30 fps, motion blur, bloom, AO, volumétricos, AgX "Medium High Contrast"; saída PNG por plano → `ffmpeg` | `vf export`/`vf qa` como hoje |

## Regras de look (medidas no estilo estudado)
- Câmera **sempre** em movimento: vida mediana ~0,12 no `vf frames` (4x o nosso cutout). Glides longos e lentos; 1 plano pode ter 3-4 enquadramentos por movimento de câmera.
- Luma média ~0,43 em todas as cenas, inclusive à noite (noite = azul-acinzentado com luzes quentes, nunca preto).
- Sol baixo (golden hour) por padrão, perspectiva aérea forte, céu com nuvens volumétricas.
- Personagens sem sombreamento próprio, com **rim light** do mundo e sombra de contato: leitura imediata no celular.
- O mundo "brilha": glints, partículas no ar, lâmpadas e reflexos.

## Ordem de implementação proposta (quando o Blender estiver instalado)
1. `brew install --cask blender` e `vf doctor` checando a versão.
2. Script `blender/kit.py`: monta um kit mínimo (chão com grama scatter, estrada, 1 árvore, HDRI Poly Haven) e mede segundos/frame no Eevee a 1080x1920.
3. `blender/elenco.py`: importa um pack de sprites como cards com troca de estado por keyframe de textura.
4. `vf-mygo -dump-actors` (ou `anim.ts` em Node): exporta as transformações por frame para o Blender aplicar.
5. `blender/plano.py`: renderiza um clipe `scene` (set + câmera + atores) para PNGs; comparar com o mesmo clipe no mygo.
6. Motor `blender-3d` no `novo.sh` + perfil no `edl-check` (vida mínima maior: ~0,08).
