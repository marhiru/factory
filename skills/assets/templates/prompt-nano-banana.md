# Prompts para o Nano Banana (Google Flow)

Prompt em inglês, frases descritivas (não lista de tags). Ordem:

1. **Cenário** (vem de `estilo.json > cenarios[<chave>]`): lugar, época, clima, quem está lá.
2. **Momentos** que o lote cobre (o `flow-lotes.mjs` junta os `visual` dos beats do cenário): "Moments to cover across the outputs (one per image, vary which one): ... | ...".
3. **Variação de enquadramento:** "Vary the framing across outputs: wide establishing shot, medium shot, close-up detail, low angle, over-the-shoulder, high angle."
4. **Sufixo de estilo** fixo do vídeo.
5. **Negativos:** "Avoid: ...".

Por que um prompt por cenário e não por plano: a cota é por prompt (20/dia) e cada prompt devolve até 24 imagens. Um lote = um cenário rende 24 variações do mesmo lugar e da mesma luz. Isso dá consistência e sobra para escolher o melhor enquadramento de cada plano. Cenário com mais de 8 planos vira 2+ lotes divididos por igual.

## Exemplo
```
A crowded 19th-century harbor at night, gas lamps reflecting on wet cobblestones, sailing ships at the docks.
Moments to cover across the outputs (one per image, vary which one): a dock worker drops a crate that splits open | a lone officer reading a telegram under a lamp | ships' masts against a stormy sky.
Vary the framing across outputs: wide establishing shot, medium shot, close-up detail, low angle, over-the-shoulder, high angle.
Cinematic documentary still, painterly realism, soft volumetric light, deep teal shadows and warm amber highlights, 35mm lens, shallow depth of field, subtle film grain, rule-of-thirds composition with breathing room for a slow zoom, no text.
Avoid: text, letters, watermark, logo, extra fingers, deformed hands, anachronistic objects.
```

## Ajustes por problema
| Problema na folha de contato | Ajuste no prompt do próximo lote |
|---|---|
| Estilo variando entre lotes | mais 1-2 `referencias`; sufixo idêntico; mesmo modelo |
| Texto/letreiros aparecendo | "no signage, blank walls" no cenário |
| Tudo em plano aberto | listar 2-3 detalhes concretos nos momentos ("close-up of the telegram in his hand") |
| Rostos estranhos | "seen from behind", "silhouette", "face in shadow" |
| Sujeito colado na borda (o zoom corta) | "centered subject with generous margins" |
