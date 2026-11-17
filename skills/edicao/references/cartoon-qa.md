# Revisão visual e entrega cartoon

Aplica ao desenho 2D e aos cards no mundo 3D. Leia depois dos relatórios de referência e antes de aceitar animação ou render final.

## Comparação e iteração

1. Escolha frames de gancho, dia/golden, close, aberto, maior movimento e payoff. Faça 1–3 comparações por rodada na mesma escala. `templates/cartoon/cartoon-stills.mjs` gera stills com bundle único; `sbs_sheet.py` precisa de Pillow e monta a folha lado a lado.
2. Avalie cor dia/golden, luz, ink, textura, personagem, profundidade e composição; dê nota por área e correção com frame e arquivo responsável. Compare com o relatório medido, não só com adjetivos. Após integrar, rode tsgo/tsrs e volte aos mesmos frames.
3. Quando houver trabalho paralelo autorizado, use sandboxes por câmera (`story.ts`), mundo (`world.ts`, `lights.ts`, `shade.ts`, `grade.ts`), material (`paint.ts`, `build.ts`), elenco (`toon.ts`, `cast.ts`, `cardshade.ts`) e pós (`ink.ts`, `Bridge.tsx`, `lensblur.ts`). Um dono por arquivo; preserve snapshot antes de integrar; sincronize todas as cópias da base após o merge. Contratos alterados (`NOINK`, `LIGHTS`, `dofAt`, etc.) precisam chegar a todos os leitores.
4. Uma revisão independente pode usar agente disponível/autorizado, sem exigir modelo ou nível específico. Se não houver delegação, faça uma rodada separada olhando os artefatos e critérios. Não importe a obrigação histórica de “Opus max”.

## Armadilhas de legibilidade

- Balanço: duas cordas, banco largo, criança entre elas e estrutura fora do arco; uma corda escondida pode sugerir enforcamento.
- Prop a menos de ~2 m da lente pode virar bloco cortado. Reconfira depois de cada mudança de câmera; bush de foreground precisa parecer arbusto, não bola verde.
- Bandeiras, uniformes e objetos precisam ser compatíveis com época/lugar; cores/símbolos podem comunicar outro país sem intenção.
- Placa pintada sob sol lateral pode ficar cinza/azul ilegível: material unlit apenas na face da placa, preservando a luz normal do elenco.
- Rim dourado externo deslocado vira moldura de chapéu; faça rim interno. Grades de varanda atravessando torsos pedem altura de câmera/layout diferente; ~2,5 m funcionou no exemplo.
- Pés no chão, sombra de contato, escala e oclusão coerentes. Piscada, passada e reação precisam ser conferidas em sequência, não só em still.
- Holds sem movimento parecem freeze; preserve atuação/câmera motivada, sem esconder o aviso com ruído ou grão.

## Operação e export

Use `REMOTION_BROWSER` para Chrome e `REMOTION_TMP` para um diretório do projeto com espaço. O helper define TMPDIR/TEMP/TMP; cada bundle copia `public/`, então mantenha só assets usados. Limpe apenas temporários pertencentes ao projeto, com renders parados e observando autorização para exclusões; não varra a pasta temporária compartilhada.

No JS, use barras `/` ou escape `\\`: `\r` num caminho vira retorno de carro. Restrinja o tsconfig à fonte; arquivos TS de rascunho dentro de diretórios incluídos podem quebrar a checagem. Stills reduzidos não provam equivalência do post em 2K: confira escala dos parâmetros em pixels e o DPR do R3F.

Render de prévia pode usar 1080 ou escala menor; final vertical 2K opcional com `--scale 1.3333` (1440×2560). Tempos de render dependem da cena; não prometa os ~4 fps medidos no exemplo. O renderer pode escrever trilha silenciosa: ao muxar áudio externo, selecione explicitamente vídeo 0 e áudio 1 (o `vf export --audio` faz a integração da fábrica).

Gere o plano de transições a partir dos tempos atuais, em **frames inteiros** para `vf qa --plan`: `shots` tem `a/b`, `flash` é `[início,fim]`, `xfade`/`planned` são listas de intervalos. O helper `scripts/cartoon-plan.mjs` importa as constantes atuais de `story.ts` com Bun; o formato em frames também é aceito pelo QA da fábrica. Exceções cobrem só transições planejadas; não mascare defeitos reais. Use `vf qa` estrito, sua folha densa e escuta do mix. O `edl-check` sozinho não comprova animação interna dos clipes 3D.

## Revisão de construção e composição

Além de look, aplique o [QA visual em cinco etapas](qa-cinco-etapas.md) para validar modelo,
contato/atuação, suporte e oclusão dos props, detalhes funcionais e margens de câmera.
Uma folha de stills confirma aparência; use clipes para contato, continuidade e extremos.
