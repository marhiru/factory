# roteiro/beats.json — formato

```json
{
  "formato": "yt-short",
  "wpm": 222,
  "beats": [
    {
      "id": 1,
      "tipo": "gancho | contexto | escalada | virada | payoff | analogia | fecho | capitulo | reganho",
      "titulo": "só em tipo=capitulo",
      "texto": "frase falada (igual ao roteiro.txt, números podem estar em algarismos aqui)",
      "visual": "o que aparece; no longo, vários planos separados por ';'",
      "camera": "close | medio | aberto | aereo | insert | pov",
      "assets": ["img/porto.jpg"],
      "sfx": ["whoosh", "sino"],
      "texto_tela": "texto curto queimado na tela (opcional)",
      "recorte": false,

      "w0": 0, "w1": 16, "start": 0.09, "end": 6.74
    }
  ]
}
```
`w0`, `w1`, `start` e `end` são escritos por `vf align` depois da voz. Não preencha à mão.
