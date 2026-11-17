# Ferramental JavaScript e TypeScript

- Use **Bun** para executar JavaScript/TypeScript (`bun arquivo.mjs`, `bun arquivo.ts`), instalar dependências (`bun install`, `bun add`, `bun add -d`) e executar scripts (`bun run --bun <script>`).
- Para CLIs já instaladas no projeto, use `bun run --bun <binário> ...`: se faltar a dependência, instale-a explicitamente. Para executar um pacote externo, use `bunx --bun <pacote> ...`.
- Para checar/compilar TypeScript, use **tsgo ou tsrs**. O template da fábrica escolhe **tsgo**, fornecido por `@typescript/native-preview`, com `bun run --bun typecheck` → `tsgo --noEmit`. Se o projeto já usa tsrs, preserve essa escolha e suas opções documentadas; não suponha que suas flags sejam iguais às do tsgo.
- Bun executa TypeScript, mas não verifica tipos. Rode o checker antes de aceitar mudanças em código TS/TSX.
- Preserve os imports `node:fs`, `node:path` etc.: são APIs compatíveis com Bun. Uma dependência transitiva de Node/TypeScript não muda o ferramental do projeto.
- Preserve `bun.lock` gerado pela instalação. Não gere lockfiles de outros gerenciadores.

Referências para manutenção: [Bun run](https://bun.sh/docs/runtime), [bunx](https://bun.sh/docs/pm/bunx), [tsgo](https://github.com/microsoft/typescript-go).
