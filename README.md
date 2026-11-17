# video-factory

Plugin do Claude Code para uma fábrica de vídeo com IA: **YouTube longo, YouTube Shorts e TikTok**.

- **Skill principal** `factory`: cria o projeto e conduz as etapas (referência → roteiro → assets/voz → animação → edição/QA → entrega), com estado em `factory.yaml`.
- **Skills de etapa** com templates: `roteiro`, `assets`, `animacao`, `edicao`, `analise`, `canal` e `metricas` (analista de métricas e metadata YouTube/TikTok).
- **CLI `vf` em Rust**: análise frame a frame, transcrição, alinhamento, mix, export e QA, todos rápidos.
- **Renderer `vf-mygo` em Go** ([MyGo render](https://mygo.run/docs/plugins/render)): desenha o `data/edl.json` na GPU (Metal), sem navegador, ~5x mais rápido que o Remotion.

## Ferramental

JavaScript/TypeScript usa **Bun** para execução, dependências e CLIs. TypeScript usa **tsgo ou tsrs**; o template Remotion inclui tsgo (`bun run --bun typecheck`). Instruções: [ferramental](skills/factory/references/ferramental.md).

## Instalar

`scripts/install.sh` compila (se preciso) e cria links em `~/.local/bin`: `vf`, `vf-mygo`, `vf-novo` (novo projeto) e `vf-render-hf`. Recompilar com `scripts/build.sh` atualiza os links sozinho.
```bash
# dependências (macOS)
brew install ffmpeg whisper-cpp yt-dlp     # ffmpeg também pode vir do nix
brew install uv                             # opcional: vf voz (narração local com Chatterbox)
# Rust (cargo) para compilar o vf
./scripts/build.sh                          # gera bin/vf (Rust) e bin/vf-mygo (Go 1.27+, baixado sozinho)
bin/vf doctor                               # confere dependências

# usar o plugin numa sessão
claude --plugin-dir ~/Projects/video-factory
# ou instalar a partir de um marketplace/repo git que contenha este diretório
```
Depois, na sessão: `/video-factory:factory novo meu-video --formato yt-short,tiktok --motor kenburns`.

### Fluxo pelo terminal
```bash
mkdir meu-video && cd meu-video
vf init type="short" platform="yt,tiktok"   # short | long | "short,long"
vf auth platform="yt,tiktok"                # opcional: rascunho/postagem/agenda + analíticos (credenciais: skills/edicao/references/plataformas.md)
claude                                      # o plugin já vem habilitado; peça o vídeo
```

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
  metricas/                      # analista de métricas YT/TikTok: auditoria, retenção × roteiro, correção de metadata (vf analytics / vf meta)
templates/projeto/               # esqueleto de cada projeto
scripts/novo.sh                  # cria projeto
scripts/build.sh                 # compila vf
tools/vf/                        # código Rust
tools/vf-studio-ext/             # extensão do Chrome: métricas do Studio para o vf (native messaging)
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
| `vf init type="short,long" platform="yt,tiktok"` | transforma a pasta atual num projeto da fábrica (esqueleto, templates do motor, som procedural, `ref/fx/` com pixelfx e pixel-splats, plugin ligado no `.claude/settings.json`); nunca sobrescreve arquivos |
| `vf auth platform="yt,tiktok" [account=…] [direct=1]` · `vf auth status` · `vf auth use` · `vf auth logout` | OAuth (PKCE, loopback) no YouTube e TikTok; tokens em `~/.config/vf/auth/` |
| `vf publish <mp4> platform=yt\|tiktok [mode=rascunho\|privado\|nao-listado\|publico] [at=…] [meta=… thumb=…]` | longo ou Short: rascunho (padrão), posta ou agenda (YouTube publishAt; TikTok pela agenda local) |
| `vf agenda [list\|run\|cancel id=\|install]` | posts agendados do TikTok (LaunchAgent a cada 5 min) |
| `vf meta <ID> platform=yt meta=… [aplicar=1]` | corrige título/descrição/tags de vídeo já enviado (prévia antes × depois) |
| `vf relatorio [video=ID] [janela=…] [filtros…]` | relatórios estilo Studio (individual e geral) em `.md`/`.json`/`.html` → `out/relatorios/` |
| `vf analytics platform=yt\|tiktok [video=ID] [dias=28] [comentarios=1] [curva=1]` | canal, uploads, Shorts × longos, retenção, tráfego, curva por idade → `out/analytics/` (skill `metricas`) |
| `vf studio instalar` · `vf studio metricas video=ID` · `vf studio mapear continuaram=…` | **extensão do Chrome** (`tools/vf-studio-ext`) que captura as métricas que o Studio carrega (ex.: "Continuaram assistindo"), sem tirar cookies do navegador; modo ativo gentil (1 página a cada 35-75 s, até 30 vídeos/dia) |
| `vf voz <texto.txt> -o public/voice/voice.wav [--ref voz.wav]` | narração **local** com Chatterbox (pt-BR, clona a voz do canal de um WAV de referência, cache por frase, `--refazer N`) |
| `vf formats` · `vf probe` · `vf doctor` | utilitários |

### Publicar, agendar, corrigir metadata e analisar

O padrão de `vf publish` é **rascunho**. Postar ou agendar só quando você pedir. Credenciais, limites e erros comuns: [plataformas.md](skills/edicao/references/plataformas.md).

**Autorizar contas** (uma vez por conta)
```bash
vf auth platform=yt account=meu-canal          # YouTube: upload, analíticos, edição de metadata e comentários
vf auth platform=tiktok account=meu-perfil     # TikTok: rascunho na caixa de entrada + analíticos
vf auth platform=tiktok account=meu-perfil direct=1   # + post direto/agendado (o app precisa do Direct Post aprovado)
vf auth platform=tiktok-biz account=meu-perfil # TikTok Business (experimental): retenção, tempo médio, alcance no vf relatorio
vf auth status                                 # contas, canal, validade; * = padrão
```
Contas autorizadas antes desta versão: rode `vf auth platform=yt account=<apelido>` de novo para ganhar a permissão de edição do `vf meta`.

**`vf publish`: longo ou Short, agora ou agendado**
```bash
vf publish out/final_yt-long.mp4  platform=yt meta=out/publicacao_yt-long.json thumb=out/thumb.jpg  # rascunho (privado)
vf publish out/final_yt-long.mp4  platform=yt mode=publico meta=out/publicacao_yt-long.json          # posta agora
vf publish out/final_yt-short.mp4 platform=yt at="2026-11-20 18:00" meta=out/publicacao_yt-short.json # agenda (hora local)
vf publish out/final_yt-short.mp4 platform=yt mode=nao-listado
vf publish out/final_tiktok.mp4   platform=tiktok                                                   # rascunho na inbox do app
vf publish out/final_tiktok.mp4   platform=tiktok mode=publico caption="Legenda #tema" capa_ms=800   # post direto
vf publish out/final_tiktok.mp4   platform=tiktok at=+3h caption="Legenda #tema"                    # agenda local
vf publish out/final_yt-short.mp4 platform=yt,tiktok account=canal-a,canal-b at="2026-11-21 12:00"   # várias plataformas/contas
```
| Opção | Valores |
|---|---|
| `mode=` | `rascunho` (padrão) · `privado` · `nao-listado` (só YouTube) · `amigos` (só TikTok) · `publico` |
| `at=` | `2026-11-20 18:00` (hora local) · `2026-11-20T18:00-03:00` · `+30m` · `+2h` · `+1d`; mínimo 15 min à frente; sem `mode=` vale `publico` |
| metadata | `meta=<json {title, description, tags, caption}>` ou `title=` `description=` `tags=a,b` `caption=` `category=` `idioma=` |
| extras | `thumb=` (YouTube) · `formato=yt-long\|yt-short\|tiktok` (padrão: detectado pelo arquivo) · `ia=0` (sem mídia sintética) · `kids=1` |
| TikTok direto | `comentarios=0` `duet=0` `stitch=0` desligam interações · `capa_ms=` frame da capa |

- **YouTube** agenda pelo `publishAt`: o vídeo sobe na hora como privado e o YouTube publica sozinho, mesmo com o computador desligado. O `vf` checa os limites (título ≤ 100, descrição ≤ 5000 bytes, tags ≤ 500), acrescenta `#Shorts` nos Shorts e avisa quando um vídeo longo não tem capítulos.
- **Projeto do Google Cloud não verificado**: o YouTube deixa como privado todo upload feito pela API, mesmo com `mode=publico` (o `vf` avisa). Para postar público pela API, peça a verificação do projeto.
- **TikTok** sem auditoria do app só aceita post direto privado (`mode=privado`).

**`vf agenda`: posts agendados do TikTok**

A API do TikTok não agenda. Com `at=`, o pedido fica em `~/.config/vf/agenda/` e é postado na hora.
```bash
vf agenda install                  # uma vez: LaunchAgent roda `vf agenda run` a cada 5 min (Linux: imprime a linha do crontab)
vf agenda                          # lista pendentes, falhos e os últimos feitos; mostra se o executor está ligado
vf agenda run                      # posta agora o que já venceu (é o que o LaunchAgent chama)
vf agenda cancel id=<id>           # cancela um pendente
vf agenda retry id=<id>            # devolve um que falhou para a fila
vf agenda uninstall                # desliga o executor (os pendentes ficam guardados)
```
O computador precisa estar ligado na hora marcada. Se estiver desligado, o post sai quando ele ligar. Não mova o arquivo de vídeo antes do post.

**`vf meta`: corrigir metadata de vídeo já enviado (YouTube)**
```bash
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json           # prévia: antes × depois, não grava
vf meta <ID> platform=yt meta=out/metricas/meta_<ID>.json aplicar=1 # grava; histórico em out/analytics/meta-<ID>-<data>.json
vf meta <ID> platform=yt title="Novo título" tags=a,b aplicar=1     # campos avulsos; o que não for passado fica igual
```
O TikTok não permite editar a legenda pela API: aplique a sugestão no app.

**`vf analytics`: métricas para a skill `metricas`**
```bash
vf analytics platform=yt                               # canal + últimos 50 uploads + top do período (28 dias)
vf analytics platform=yt account=all dias=90           # todas as contas, 90 dias
vf analytics platform=yt video=ID1,ID2 comentarios=1   # por vídeo: retenção, tráfego, curva por idade, comentários
vf analytics platform=yt curva=1 max=100               # views acumuladas d1/d3/d7/d28 dos 25 principais
vf analytics platform=tiktok max=60                    # perfil + posts públicos com engajamento
```
| Opção | O que faz |
|---|---|
| `dias=` | período da Analytics API (padrão 28) |
| `max=` | quantos uploads/posts listar (padrão 50) |
| `video=` | só estes vídeos, com retenção completa, tráfego e curva por idade |
| `retencao=N` | curva de retenção dos N principais (padrão 5) |
| `curva=1` | views por dia desde a publicação (para comparar vídeos na mesma idade) |
| `comentarios=1` | comentários dos principais, sem nome de autor |

Saída: `out/analytics/<plataforma>-<conta>-<data>.json`, com `videos[]` (metadata, Short × longo, views em tempo real, período, retenção, idade), `medianas` do canal, `por_tipo`, `trafego` e `avisos` (que explicam resultado vazio: Analytics atrasa 24-72 h, vídeo privado não soma views, API desligada). No TikTok a API só lista posts públicos e não dá retenção, tráfego nem comentários.

**`vf relatorio`: relatórios estilo Studio, por vídeo e de todos com filtros**
```bash
vf relatorio video=ID                                  # um vídeo: d1, d3, d7, d28 e total lado a lado + mediana do canal
vf relatorio video=ID janela=d7 beats=roteiro/beats.json   # uma janela; quedas de retenção cruzadas com o roteiro
vf relatorio                                           # todos os vídeos públicos, 7 primeiros dias de cada
vf relatorio tipo=short desde=2026-10-01 ordenar=retencao limite=20
vf relatorio busca="império" tag=brasil min_views=100 janela=d28 ordenar=intencionais ordem=asc
vf relatorio janela=2026-11-01..2026-11-15 account=all # faixa de datas fixa, todas as contas
vf relatorio platform=tiktok ordenar=engajamento       # TikTok (totais desde a publicação)
```
Cada relatório sai em `out/relatorios/` como `.md`, `.json` e `.html` (página com curva de retenção ou barras de retenção por vídeo, tabelas que ordenam com um clique, modo claro e escuro; `html=0` desliga). Linhas como no Studio: visualizações, intencionais, duração média, % visualizada, **retenção média** (média da curva), horas de exibição, gostei, inscritos (ganhos − perdidos), comentários e compartilhamentos, com a mediana do canal (mesmo tipo, mesma janela) e a diferença.

| Opção | Valores |
|---|---|
| `janela=` | `d1` · `d3` · `d7` (padrão do geral) · `d28` · `dN` · `total` · `todas` (padrão do individual) · `AAAA-MM-DD..AAAA-MM-DD` |
| filtros | `tipo=short\|longo` · `privacidade=public\|unlisted\|private` · `desde=`/`ate=` (data de publicação) · `busca=` (título/descrição) · `tag=` · `min_views=` |
| ordem | `ordenar=views\|intencionais\|retencao\|avd\|pct\|horas\|inscritos\|likes\|comentarios\|shares\|engajamento\|publicado` · `ordem=desc\|asc` · `limite=N` · `max=200` (uploads lidos) |

Limites do que a API entrega, avisados no próprio relatório:
- **"Continuaram assistindo"** (Shorts: assistiram × deslizaram) não existe na API pública; o Studio calcula com dados internos do feed. Aparece como "—", a não ser com `studio=1` e dados capturados pela extensão do Chrome `vf studio` (não oficial; ver [plataformas.md](skills/edicao/references/plataformas.md#youtube-studio-extensão-do-chrome-vf-studio-não-oficial)).
- A janela `dN` é por dia (a Analytics não tem hora): `d1` = dia da publicação. Vídeo mais novo que a janela sai "parcial" e fica fora da mediana.
- A Analytics atrasa 24-72 h. Até lá, visualizações, gostei e comentários vêm em tempo real da Data API e o resto fica "—".
- **TikTok**: visualizações, curtidas, comentários, compartilhamentos, duração e engajamento pela API normal. Tempo médio, **retenção média**, "assistiram até o fim", alcance e curva de retenção só com uma conta **TikTok Business** (Business ou Creator) autorizada em `vf auth platform=tiktok-biz` (experimental, ver [plataformas.md](skills/edicao/references/plataformas.md#tiktok-business-retenção-experimental)).

**Skill `metricas`** (na sessão do Claude): `/video-factory:metricas relatorio [<ID>] | auditoria | upload <ID> | retencao <ID> | metadata <ID> | comentarios <ID> | pauta`. No fluxo `relatorio`, ela gera o relatório com `vf relatorio`, comenta os números e, se você pedir, publica o HTML como página privada para compartilhar. Ela roda o `vf analytics`, compara cada vídeo com a mediana do próprio canal e cruza as quedas de retenção com o roteiro (`skills/metricas/scripts/cruzar.py`). Também sugere título, descrição, tags e legenda com antes × depois, e só aplica com `vf meta aplicar=1` depois do seu OK.

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
