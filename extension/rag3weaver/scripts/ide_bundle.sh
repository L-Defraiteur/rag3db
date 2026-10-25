#!/bin/bash
# **Le bundle de l'IDE en React** (page 08 du 25 octobre 2026) : les sources sous
# ui/ide (TSX, shadcn/ui copiés, Tailwind), bâties par esbuild et tailwindcss, écrites
# sous templates/ide/app/{app.js, app.css, bundle.json} — commitées, embarquées par
# build.rs comme tout templates/, servies par l'hôte (/ide/app.js, /ide/app.css).
# Le bâti Rust n'a pas besoin de node ; ce script, si (node 22, npm ci sur le lock).
#
#   scripts/ide_bundle.sh           bâtit et écrit le bundle (à commiter avec ses sources)
#   scripts/ide_bundle.sh --check   rebâtit à part et compare octet pour octet au commité :
#                                   ROUGE si le bundle servi n'est pas celui du code
set -euo pipefail
ici=$(cd "$(dirname "$0")/.." && pwd)
cd "$ici/ui/ide"
npm ci --no-audit --no-fund --ignore-scripts --silent
exec node build.mjs "$@"
