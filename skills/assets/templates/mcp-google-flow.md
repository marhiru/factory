# MCP do Google Flow (Nano Banana): imagens dos assets

A fábrica gera as imagens pelo Google Flow (labs.google/flow) com a conta do usuário, via MCP. O plugin declara o servidor `google-flow` no `.mcp.json`: [`@park-sang-gwon/google-flow-mcp`](https://github.com/ParkSangGwon/google-flow-mcp), versão fixada em 0.3.1.

> Servidor de terceiros, **não oficial**: ele automatiza o Flow num Chrome dedicado. O Flow não tem API pública, então uma mudança no site pode quebrar a automação. Ele gasta créditos do Flow como um clique em "Gerar".

## Instalação (uma vez)
1. Node 20+ e Google Chrome.
2. Login: `bunx --bun @park-sang-gwon/google-flow-mcp@0.3.1 doctor`. Abre um Chrome com perfil próprio em `~/.google-flow-chrome`; entre na conta Google com acesso ao Flow. O `doctor` precisa mostrar `flow: logged in`.
3. Com o plugin carregado, o servidor sobe sozinho. Fora do plugin: `claude mcp add google-flow -- bunx --bun @park-sang-gwon/google-flow-mcp@0.3.1`.
4. Confira: a ferramenta `flow_connect` precisa responder `logged_in: true`.

## Cota (regra da fábrica)
- **20 prompts por dia, até 24 imagens por prompt.** O que custa é o prompt: **todo pedido sai com `count` = máximo (24)**. Em um dia são 480 imagens, nunca 20.
- A cota é **global** (todos os projetos): `~/.cache/vf/flow-cota.json`, controlada pelo `skills/assets/scripts/flow-lotes.mjs`.
- Se o Flow devolver menos imagens que o pedido (limite do plano ou do site), o `registrar` avisa. Ajuste com `flow-lotes.mjs cota --max-por-prompt N` para o plano dos lotes seguintes refletir o real.

## Loop de geração (a skill `assets` segue isto)
```
FL=${CLAUDE_PLUGIN_ROOT}/skills/assets/scripts/flow-lotes.mjs
bun $FL plan                 # beats.json + assets/flow/estilo.json -> assets/flow/lotes.json
bun $FL proximo              # args prontos do próximo lote (sai 3 se a cota do dia acabou)
#  -> chamar o MCP: flow_generate_image(prompt, model, ratio, count, reference_images, output_dir, auto_confirm: true)
#     se voltar job em andamento: flow_generate_image(resume: true, job_id)
bun $FL registrar <lote> assets/flow/_entrada/<lote>   # conta 1 prompt e move as imagens
${CLAUDE_PLUGIN_ROOT}/bin/vf sheet assets/flow/<lote>   # folha de contato para a curadoria
```
Repita até `proximo` responder `fim` ou `cota_esgotada`. Com a cota esgotada, marque `etapas.assets: aguardando_cota` e `notas: retomar em <data>` no `factory.yaml` e pare. O `factory continuar` retoma no dia seguinte.

## Primeiro lote = lote de estilo
Antes dos lotes do roteiro, gere 1 lote só para o estilo (o cenário mais importante, sem referências). Escolha 3-6 imagens que definem o look, salve em `assets/ref/estilo-NN.jpg` e liste em `estilo.json > referencias`. Isso gasta 1 prompt e economiza retrabalho em todos os outros.

## Erros comuns do MCP
`BUSY` (outra geração rodando: espere e `resume`), `POLICY_BLOCKED` (reescreva o cenário sem o termo sensível), `MODEL_NOT_AVAILABLE` (troque `modelo` para `nano-banana-2`), `DOWNLOAD_FAILED` (`flow_media_download` pelos `media_ids`).
