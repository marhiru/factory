# video-factory

Plugin do Claude Code para uma fábrica de vídeo com IA: **YouTube longo, YouTube Shorts e TikTok**.

- **Skill principal** `factory`: cria o projeto e conduz as etapas (referência → roteiro → assets/voz → animação → edição/QA → entrega), com estado em `factory.yaml`.
- **Skills de etapa** com templates: `roteiro`, `assets`, `animacao`, `edicao`, `analise`.
- **CLI `vf` em Rust**: análise frame a frame, transcrição, alinhamento, mix, export e QA, todos rápidos.
- **Renderer `vf-mygo` em Go** ([MyGo render](https://mygo.run/docs/plugins/render)): desenha o `data/edl.json` na GPU (Metal), sem navegador, ~5x mais rápido que o Remotion.

## Ferramental

JavaScript/TypeScript usa **Bun** para execução, dependências e CLIs. TypeScript usa **tsgo ou tsrs**; o template Remotion inclui tsgo (`bun run --bun typecheck`). Instruções: [ferramental](skills/factory/references/ferramental.md).

## Instalar

`scripts/install.sh` compila (se preciso) e cria links em `~/.local/bin`: `vf`, `vf-mygo`, `vf-novo` (novo projeto) e `vf-render-hf`. Recompilar com `scripts/build.sh` atualiza os links sozinho.
```bash
# dependências (macOS)
brew install ffmpeg whisper-cpp yt-dlp     # ffmpeg também pode vir do nix
# Rust (cargo) para compilar o vf
./scripts/build.sh                          # gera bin/vf (Rust) e bin/vf-mygo (Go 1.27+, baixado sozinho)
bin/vf doctor                               # confere dependências

# usar o plugin numa sessão
claude --plugin-dir ~/Projects/video-factory
# ou instalar a partir de um marketplace/repo git que contenha este diretório
```
Depois, na sessão: `/video-factory:factory novo meu-video --formato yt-short,tiktok --motor kenburns`.

## Estrutura
```
.claude-plugin/plugin.json
formats.yaml                     # specs de yt-long / yt-short / tiktok (fonte única; embutida no vf)
skills/
  factory/                       # MAIN
  roteiro/templates/             # yt-long, short, tiktok, ganchos, beats.schema, SCRIPT
  assets/templates/              # style-bible, manifest, voz-elevenlabs, sfx-musica, fontes-cc0
  animacao/templates/            # remotion-2d/ (template Remotion orientado a edl.json), edl.schema, kenburns, ia-video, remotion-three
  animacao/scripts/beats2edl.mjs # beats com tempo -> primeiro corte (edl.json)
  edicao/templates/              # recorte (longo -> shorts), publicacao
  analise/                       # estudo de referências com vf analyze
templates/projeto/               # esqueleto de cada projeto
scripts/novo.sh                  # cria projeto
scripts/build.sh                 # compila vf
tools/vf/                        # código Rust
tests/synthetic.sh               # teste de integração (cortes em frames conhecidos)
```

## `vf`
| Comando | O que faz |
|---|---|
| `vf analyze <arquivo\|url> -o ref/x --lang pt` | download, cortes e crossfades, keyframes, contact sheets, transcrição e ritmo → `analysis.md`/`.json` |
| `vf frames <arq>` | estatística por frame (luma, croma, movimento, histograma) + `shots.json` com `xfade` |
| `vf keyframes <arq> [--shots s.json] [--every N]` | keyframes + contact sheets com timecode |
| `vf transcribe <audio\|video> -o data --lang pt [--dtw] [--prompt ...]` | whisper.cpp (Metal) → `words.json`, `.srt`, `.txt`; `--elevenlabs` converte JSON do Scribe |
| `vf align roteiro/beats.json data/words.json` | amarra beats às palavras (start/end/w0/w1) |
| `vf mix data/edl.json -o public/audio/mix.wav` | voz + música com ducking + SFX numa passada |
| `vf export <video> -f <formato> -o out.mp4 --audio mix.wav [--fit pad\|crop\|blur] [--fast]` | enquadramento, encode, loudnorm em 2 passadas, mux |
| `vf qa <final.mp4> -f <formato> -o out/qa [--strict]` | specs, loudness, pretos/brancos/congelados/pops, planos, sheets densas |
| `vf formats` · `vf probe` · `vf doctor` | utilitários |

### Desempenho (Apple M4, 10 núcleos)
| Tarefa | Entrada | Tempo |
|---|---|---|
| `vf frames` | 110 s, 1440x2560 h264 (3309 frames) | 4,3 s (~765 fps) |
| `vf transcribe` | 110 s de narração PT | 7,2 s (large-v3-turbo q5, Metal) |
| `vf analyze` completo | mesmo vídeo | 11,5 s (decode e transcrição em paralelo) |
| `vf qa` | mesmo vídeo | ~6 s (o `short_qa.py` levava 15,7 s) |
| `vf keyframes` | 33 planos em 2K | 3,0 s |
| `vf mix` | 3 faixas, 30 s | 0,07 s |

Notas de desempenho:
- O decode por software com threads do ffmpeg foi ~5x mais rápido que o VideoToolbox para análise (o gargalo é trazer cada frame da GPU). Por isso o hardware ficou opcional (`--hw`).
- O decode sai direto no tamanho de análise (256 px de largura), e toda a estatística é calculada numa passada em inteiros.
- Vídeos longos são divididos em segmentos paralelos (`-j`) com 2 frames de sobreposição: o resultado é idêntico ao serial (verificado em `tests/synthetic.sh`).

## Animação cutout 2D (padrão)
Todo vídeo da fábrica é animado. Personagens e props viram sprites (expressões + piscada + ciclos), exportados de desenhos Canvas2D por `skills/animacao/templates/sprites/`. Os cenários são plates sem personagens. Cada plano é um clipe `scene` no `edl.json`, com câmera e parallax, atores com keyframes, estados, `idle`, entrada e saída, e ações em `at_word`. A matemática é idêntica no mygo e no Remotion (SSIM 0,996 no teste).

Trava: `skills/animacao/scripts/edl-check.mjs` reprova um EDL com menos de 70% de animação real ou com um trecho acima de 6 s sem animação. Ken Burns e cartelas não contam. Exemplo de cenas: `skills/animacao/templates/cenas/cenas.exemplo.py`.

## Renderers do `edl.json`: MyGo × Remotion
Mesmo `edl.json` (22,2 s, 666 frames, Ken Burns + texto + legenda karaokê), Apple M4:

| | vf-mygo | Remotion 4 |
|---|---|---|
| 1080x1920 | **3,4 s** (198 fps), 0,9 s de CPU | 16,7 s, 53,6 s de CPU |
| 1440x2560 (2K) | **6,6 s** | 23,4 s |
| Clipes de vídeo 1080p (trim, 2x) | **1,7 s**, 150/150 frames exatos | 6,5 s, 149/150 |
| Igualdade visual | SSIM 0,982; imagens a ~41 dB de PSNR; difere só na quebra de linha de legendas no limite da largura | referência |
| Saída | H.264 yuv420p (o export copia sem recodificar) | yuvj420p (o export recodifica) |

O `vf-mygo` cobre os clipes `image`, `video`, `text` e `color`, as transições e as legendas. Componentes React sob medida, three.js e o Studio continuam no Remotion. Use a mesma fonte (`edl.json > font`) nos dois.

## Vídeo longo estático com tema (motor `estatico`, 10-15 min)
Imagens caprichadas do **Google Flow (Nano Banana) via MCP**, zoom-in leve, cards tipográficos, acabamento de filme e roteiro com esquemática de hook rate.

| Etapa | Ferramenta | Trava |
|---|---|---|
| Roteiro | `skills/roteiro/templates/yt-long-estatico.md` | `roteiro-check.mjs` (gancho sem preparação, promessa/loop até 0:40, re-hooks, capítulos com loop, payoff, fecho) |
| Imagens | MCP `google-flow` (`.mcp.json`) + `flow-lotes.mjs` | 1 prompt = 24 imagens; 20 prompts/dia; cota global em `~/.cache/vf/flow-cota.json` |
| Curadoria | `vf sheet` + `curadoria.mjs` | upscale abaixo de 2150 px; manifest com origem |
| Edição | `roteiro/cenas.py` (`estatico.py`) | `edl-check` perfil estático (movimento perceptível, planos 1,5-9 s, interrupção ≤ 40 s, sem repetição em 120 s, grão + vinheta) |
| Render | HyperFrames (`edl2hf.mjs` + `render-hf.sh`) | `vf qa` estrito |

## Renderer HyperFrames (vídeos longos)
`skills/animacao/scripts/edl2hf.mjs` compila o mesmo `data/edl.json` numa composição HTML + GSAP (sem CDN, telemetria desligada no `render-hf.sh`). Suporta image (zoom/pan/camadas), video, text, color, `card` (capítulo, dado, citação, mapa, comparação), transições (fade, blur, dip, flash, slide, punch) e `overlays` (grão, vinheta, light leak, graduação). Medido no M4: 60 s de 1080p em 39 s; paridade com o mygo em Ken Burns com SSIM ≥ 0,99. Clipes `scene` (cutout) ficam no mygo.

## QA estrito
`vf qa` é estrito por padrão: WARN = FAIL, código 1. Limites por formato no bloco `qa:` do `formats.yaml` (duração de entrega, vida mínima de plano, plano máximo, bitrate, abertura do áudio, ar morto, buracos na voz, gancho visual, repetição de imagem, barras pretas, clipping). Exceção só em `factory.yaml > qa_excecoes` com motivo. `--lenient` para rascunho.

## Esteira, canal e glossários
- **Esteira** (`skills/factory/templates/esteira.md`): postos, passes incrementais (animatic → layout → animação → fx → som → final) e sub-agentes por lote de **cartões de plano** (`skills/factory/scripts/planos.mjs`), cada um dono só dos seus arquivos (`roteiro/cenas/sNN.py`, juntados por `load_parts()`).
- **Canal** (`skills/canal`): `canal.yaml` (nicho, promessa, formato, voz, elenco, marcas próprias), pauta com nota, checklist de criação e revisão semanal de métricas. `novo.sh --canal` aplica o kit do canal no projeto.
- **Glossários**: `skills/animacao/templates/glossario-fx.md` (animação, câmera, montagem, look, FX, som) e `skills/roteiro/templates/glossario-roteiro.md` (estrutura, engrenagens, retenção, métricas).
- **Mundo 3D no Blender** (especificação, ainda não implementado): `skills/animacao/templates/mundo-3d-blender.md`.

## Testes
```bash
cd tools/vf && cargo test --release     # unitários (cortes, transcrição, align, formats)
tests/synthetic.sh                       # integração: cortes exatos em 60/150/240, paralelo == serial, qa, export
```

## Cartoon 2D em mundo 3D

Os conceitos do [claude-animated-shorts](https://github.com/mromidev/claude-animated-shorts) estão distribuídos nas sete skills: desenho/atuação 2D, mundo 3D, luz, materiais, câmera, pós, roteiro, som e QA. Origem fixada e licença MIT em [ORIGEM.md](skills/animacao/templates/remotion-three/ORIGEM.md).

`./scripts/novo.sh <pasta> --motor remotion-three --formato yt-short --install` cria uma demonstração técnica executável com Bun/tsgo, fonte do mundo e composição `CartoonShort`. O desenho Canvas2D também pode ser exportado como sprites para mygo. Crie história, elenco e layout próprios antes de produzir.

Templates incorporados: [mundo e elenco](skills/animacao/templates/remotion-three/), [áudio procedural](skills/assets/templates/audio-procedural/), [stills e comparação](skills/edicao/templates/cartoon/). GLBs Kenney e frames privados de referência não estão incluídos.
