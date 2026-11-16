# roteiro/beats.json — formato

```json
{
  "formato": "yt-short",
  "wpm": 222,
  "beats": [
    {
      "id": 1,
      "tipo": "gancho | promessa | loop | contexto | escalada | pergunta | virada | revelacao | reganho | capitulo | payoff | analogia | fecho",
      "titulo": "só em tipo=capitulo",
      "texto": "frase falada (igual ao roteiro.txt, números podem estar em algarismos aqui)",
      "visual": "o que aparece; no longo, vários planos separados por ';'",
      "cenario": "chave do lugar/situação (motor estatico: agrupa as imagens nos lotes do Google Flow)",
      "card": { "kind": "dado | citacao | mapa | comparacao", "...": "dados do card (opcional; interrupção de padrão)" },
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

Tipos que a trava `roteiro-check.mjs` usa: `gancho` (1º beat), `promessa` (até 0:40), `loop` (pergunta aberta), `reganho`/`virada`/`revelacao` (re-hooks), `pergunta`, `capitulo` (com `titulo`), `payoff`, `fecho` (último).
