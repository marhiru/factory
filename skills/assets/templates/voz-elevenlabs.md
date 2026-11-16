# Voz com ElevenLabs

## Preparar o texto (`roteiro/roteiro.txt` → `voice/texto_elevenlabs.txt`)
- Uma frase por linha, sem marcações de cena.
- Números, datas e siglas **por extenso** como devem soar ("mil oitocentos e oitenta e nove", "ó-ene-u").
- Pausas: pontuação, não "...". Para uma pausa explícita: `<break time="0.4s" />` (no máximo 2 por minuto).
- Palavras estrangeiras com grafia fonética se a voz errar.

## Gerar
**Com API** (se existir `ELEVENLABS_API_KEY` no ambiente, nunca peça a chave no chat):
```
curl -sS -X POST "https://api.elevenlabs.io/v1/text-to-speech/$VOICE_ID?output_format=pcm_44100" \
  -H "xi-api-key: $ELEVENLABS_API_KEY" -H "Content-Type: application/json" \
  -d @- <<JSON > public/voice/voice.pcm
{"text": $(jq -Rs . < voice/texto_elevenlabs.txt), "model_id": "eleven_multilingual_v2",
 "voice_settings": {"stability": 0.5, "similarity_boost": 0.75, "style": 0.0, "speed": 1.0}}
JSON
ffmpeg -f s16le -ar 44100 -ac 1 -i public/voice/voice.pcm -ar 48000 public/voice/voice.wav && rm public/voice/voice.pcm
```
Textos longos (YouTube longo): gere por capítulo (`voice/cap01.wav`…) e concatene com `ffmpeg -f concat`, mantendo `voice_id` e settings iguais.

**Sem API**: entregue ao usuário o `voice/texto_elevenlabs.txt`, as settings do `factory.yaml > voz` e o caminho onde salvar (`public/voice/voice.wav`). Esta é uma das poucas pausas permitidas depois do roteiro aprovado.

## Settings de partida
| Uso | stability | similarity | style | speed |
|---|---|---|---|---|
| Short narrado rápido, tom único | 0.45-0.55 | 0.75 | 0.0 | 1.05-1.1 |
| Documentário longo | 0.55-0.65 | 0.8 | 0.1 | 1.0 |
| Personagem expressivo | 0.3-0.4 | 0.75 | 0.3-0.5 | 1.0 |

## Depois de gerar
1. Corte o silêncio inicial: a voz deve começar em ≤ 0,1 s nos shorts.
   `ffmpeg -i in.wav -af "silenceremove=start_periods=1:start_threshold=-45dB" out.wav`
2. `$VF transcribe public/voice/voice.wav -o data --lang pt`
3. Confira a velocidade: palavras ÷ duração deve ficar perto do `wpm_alvo`. Se passar de ±8%, ajuste `speed` e gere de novo.
4. `$VF align roteiro/beats.json data/words.json`
5. Registre no manifest: `voice_id`, modelo, settings e data.
