# Voz local com Chatterbox

Narração gerada **na máquina**, sem API nem custo por caractere: [Chatterbox](https://github.com/resemble-ai/chatterbox)
(Resemble AI, MIT) com o ajuste fino oficial para **português do Brasil**
([`ResembleAI/Chatterbox-Multilingual-pt-br`](https://huggingface.co/ResembleAI/Chatterbox-Multilingual-pt-br)).
Alternativa ao ElevenLabs ([voz-elevenlabs.md](voz-elevenlabs.md)); escolha em `factory.yaml > voz.provedor`.

| | Chatterbox (local) | ElevenLabs (API) |
|---|---|---|
| Custo | zero por geração | por caractere |
| Voz do canal | clonada de um WAV de referência (`ref`) | `voice_id` da conta |
| Qualidade pt-BR | boa com referência pt-BR limpa; revise pronúncia de nomes | muito boa |
| Velocidade (Apple Silicon) | minutos por minuto de áudio (MPS; cai para CPU se falhar) | segundos |
| Internet | só no 1º uso (baixa ~3-4 GB de modelo) | sempre |
| Marca d'água | PerTh imperceptível em todo áudio | — |

## Requisitos (uma vez)
- `uv` (`brew install uv`). O `vf voz` cria um ambiente Python 3.11 isolado com `chatterbox-tts==0.1.7`;
  os modelos vão para `~/.cache/huggingface` e a montagem pt-BR para `~/.cache/vf/chatterbox-pt-br`.
- `ffmpeg` (já obrigatório na fábrica).

## A voz do canal: o WAV de referência
A voz é a marca: **uma referência por canal**, guardada no kit (`<canal>/kit/voz-ref.wav`) e nunca trocada
depois de publicar.
- 10-20 s de fala corrida em **pt-BR**, uma pessoa só, sem música, sem eco, sem ruído, volume estável.
- Use a própria voz ou uma voz com **autorização por escrito** do dono. Nunca clone a voz de terceiros
  (famosos, narradores de outros canais) sem consentimento.
- O tom da referência vira o tom da narração: grave no ritmo e energia que o canal quer.
- Sem `ref`, o modelo usa a voz embutida (inglesa) e o sotaque vaza: só para teste.

## Preparar o texto (`roteiro/roteiro.txt` → `voice/texto.txt`)
- **Uma frase por linha**; linha em branco = pausa de parágrafo (0,6 s; entre frases, 0,25 s).
- Números, datas e siglas **por extenso** como devem soar ("mil oitocentos e oitenta e nove", "ó-ene-u").
- Frases de até ~250 caracteres (o script quebra as maiores em vírgulas; o modelo gera até ~40 s por vez).
- Nomes estrangeiros com grafia fonética se a voz errar. Linhas começando com `#` são ignoradas.

## Gerar
```
$VF voz voice/texto.txt -o public/voice/voice.wav                 # usa factory.yaml > voz
$VF voz voice/texto.txt -o public/voice/voice.wav --ref ~/Projects/canais/x/kit/voz-ref.wav --seed 42
$VF voz voice/texto.txt -o public/voice/voice.wav --refazer 3,7    # tomada nova só das frases 3 e 7
```
- Cada frase fica em cache (`voice/chatterbox/NNN-<hash>.wav`): editar uma linha regera só ela.
- `voice/chatterbox/pedacos.json` tem início/fim de cada frase: ouça, ache a frase ruim, `--refazer N`.
- `--seed` fixa a entonação: mesma semente + mesmo texto = mesmo áudio.

## Ajustes de partida
| Uso | exaggeration | cfg | temperature | tempo |
|---|---|---|---|---|
| Short narrado, tom único | 0.5 | 0.5 | 0.8 | 1.05-1.1 |
| Documentário longo | 0.4-0.5 | 0.5 | 0.7 | 1.0 |
| Expressivo/dramático | 0.7-1.0 | 0.3 | 0.8 | 1.0 |
| Referência fala rápido | 0.5 | 0.3 | 0.8 | 1.0 |
- `exaggeration` alto = mais emoção e fala mais rápida; compense baixando `cfg`.
- `cfg` 0 tira o sotaque da referência quando ela não é pt-BR.
- `tempo` muda a velocidade sem mudar o tom (atempo do ffmpeg); prefira acertar o ritmo com `cfg` antes.

Registre os ajustes no projeto (o `vf voz` lê daqui o que não vier na linha de comando):
```yaml
voz:
  provedor: chatterbox
  chatterbox: { ref: ~/Projects/canais/x/kit/voz-ref.wav, modelo: pt-br, exaggeration: 0.5, cfg_weight: 0.5, temperature: 0.8, seed: 42, tempo: 1.0 }
```

## Depois de gerar (igual ao ElevenLabs)
1. Ouça inteiro. Erro de pronúncia: reescreva a grafia na linha e rode de novo (só ela é regerada). Frase
   com entonação estranha: `--refazer N`.
2. `$VF transcribe public/voice/voice.wav -o data --lang pt` e confira a velocidade contra o `wpm_alvo`
   (±8 %: ajuste `cfg`/`exaggeration` ou `tempo`).
3. `$VF align roteiro/beats.json data/words.json`.
4. Registre no manifest: provedor `chatterbox`, modelo, ref (arquivo e autorização), ajustes, semente e data.
   Divulgação de IA na publicação: voz sintética (ver `edicao/templates/publicacao.md`).

## Problemas comuns
| Sintoma | Causa | Solução |
|---|---|---|
| "MPS falhou; usando CPU" | operação sem suporte no MPS desta versão do torch | normal; fica mais lento, mesmo resultado |
| sotaque estrangeiro | sem `ref` ou `ref` em outro idioma | referência pt-BR; ou `--cfg 0` |
| frase cortada ou repetida no fim | frase longa demais | quebre a linha; `--refazer N` |
| fala corrida demais | `exaggeration` alto ou referência rápida | `--cfg 0.3`, `--exaggeration 0.4` |
| primeira execução demora | download do ambiente e dos modelos (~3-4 GB) | só na primeira vez |
