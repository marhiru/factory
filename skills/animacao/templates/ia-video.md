# Planos para geradores de vídeo IA

## Bloco de consistência (copie igual em TODO prompt do vídeo)
```
STYLE: <da style bible: meio (ex. "35mm film still", "3D clay animation", "flat 2D vector")>, <paleta>, <luz>
CHARACTER <nome>: <descrição física fixa: idade, cabelo, roupa, cor dominante>, same face in every shot
NEGATIVE: text, captions, watermark, logo, extra fingers, distorted hands, morphing faces
```
Use imagem de referência do personagem (image-to-video) sempre que o provedor aceitar, e mantenha seed e modelo fixos no vídeo inteiro.

## `data/shots.yaml` (um item por plano)
```yaml
- id: b03-1
  beat: 3
  dur: 4            # s; gere 1-2 s a mais e corte com trim
  aspect: "9:16"
  camera: "slow dolly in, eye level, 35mm"
  subject: "<CHARACTER protagonista> getting out of bed, coughing"
  action: "sits up slowly, reaches for military coat"
  setting: "dim 1889 bedroom, oil lamp, Rio de Janeiro"
  light: "warm lamp key light, cold blue window fill"
  ref_image: assets/ref/protagonista.png
  prompt: |
    <STYLE>. <CHARACTER>. Slow dolly in at eye level: an old marshal sits up in bed, coughing, reaches for his coat.
    Dim 1889 bedroom lit by an oil lamp, cold blue light from the window. <NEGATIVE>
  provider: ""      # veo | kling | runway | luma | hailuo...
  seed: 1234
  status: pendente  # pendente | gerado | aprovado | rejeitado
  file: clips/b03-1.mp4
```

## Regras
- Um movimento de câmera e uma ação por plano. Prompt com duas ações vira morph.
- Descreva o plano como diretor de fotografia: lente, altura, movimento, luz. Nada de adjetivos vagos ("épico", "incrível").
- Nada de texto dentro do vídeo gerado: texto entra como clipe `text` no EDL.
- Gere 2-3 variações por plano e aprove olhando o clipe inteiro (`$VF keyframes clip.mp4 --every 0.5` mostra um contact sheet de cada).
- Planos de 3-5 s. Para mais tempo, corte para outro ângulo em vez de estender.
- Registre provedor, modelo, seed e prompt final no manifest (a reprodutibilidade importa).
