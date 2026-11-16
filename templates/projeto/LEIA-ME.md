# __NOME__ (video-factory)

Projeto criado pela skill `factory`. Estado em `factory.yaml`.

| Pasta | Conteúdo |
|---|---|
| `SCRIPT.md` | fatos + fontes + roteiro |
| `roteiro/` | `roteiro.txt` (texto da voz), `beats.json` (beats com visual; `vf align` adiciona tempos) |
| `assets/` | `manifest.yaml` (origem/licença de tudo), `style-bible.md` |
| `public/` | tudo que o render usa: `voice/`, `audio/`, `img/`, `clips/` |
| `data/` | `words.json` (tempo por palavra), `edl.json` (timeline) |
| `ref/` | análises de vídeos de referência (`vf analyze`) |
| `src/` | composição Remotion (quando o motor é Remotion) |
| `out/` | renders, QA e entregas |

Comandos (VF = caminho do binário `vf` do plugin):
```
$VF transcribe public/voice/voice.wav -o data --lang pt
$VF align roteiro/beats.json data/words.json
npx remotion studio src/index.ts
npx remotion render src/index.ts yt-short out/video.mp4 --codec h264 --crf 16
$VF export out/video.mp4 -f yt-short -o out/final_yt-short.mp4 --audio public/audio/mix.wav
$VF qa out/final_yt-short.mp4 -f yt-short -o out/qa_yt-short
```
