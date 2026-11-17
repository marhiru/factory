#!/usr/bin/env bash
# Compila as ferramentas nativas: `vf` (Rust) e, se houver Go, o renderer `vf-mygo`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/bin"
# rm antes de copiar: sobrescrever o binário no mesmo inode faz o macOS matar o processo (SIGKILL, código 137)
(cd "$ROOT/tools/vf" && cargo build --release && rm -f "$ROOT/bin/vf" && cp target/release/vf "$ROOT/bin/vf")
"$ROOT/bin/vf" --version
if command -v go >/dev/null 2>&1; then
  # GOTOOLCHAIN=auto baixa o Go 1.27+ que o MyGo pede, se o instalado for mais antigo
  (cd "$ROOT/tools/vf-mygo" && GOTOOLCHAIN=auto go build -trimpath -o "$ROOT/bin/vf-mygo" .)
  echo "vf-mygo ok"
else
  echo "go não encontrado: vf-mygo (renderer MyGo) não compilado; o Remotion continua funcionando"
fi
