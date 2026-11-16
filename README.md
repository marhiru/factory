# video-factory

Plugin do Claude Code para uma fábrica de vídeo com IA: **YouTube longo, YouTube Shorts e TikTok**.

- **Skill principal** `factory`: cria o projeto e conduz as etapas (referência → roteiro → assets/voz → animação → edição/QA → entrega), com estado em `factory.yaml`.
- **Skills de etapa** com templates: `roteiro`, `assets`, `animacao`, `edicao`, `analise`.
- **CLI `vf` em Rust**: análise frame a frame, transcrição, alinhamento, mix, export e QA, todos rápidos.
- **Renderer `vf-mygo` em Go** ([MyGo render](https://mygo.run/docs/plugins/render)): desenha o `data/edl.json` na GPU (Metal), sem navegador, ~5x mais rápido que o Remotion.

## Instalar
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

## Testes
```bash
cd tools/vf && cargo test --release     # unitários (cortes, transcrição, align, formats)
tests/synthetic.sh                       # integração: cortes exatos em 60/150/240, paralelo == serial, qa, export
```
