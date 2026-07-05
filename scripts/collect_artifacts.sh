#!/usr/bin/env bash
set -euo pipefail

DEST=${1:-otherfiles}

EXCLUDES=("./${DEST}" "./target" "./alsh-std/impl/rust/target" "./.git")

EXTS=(ll s o bc a out so obj d)

PRUNE_EXPR=()
for p in "${EXCLUDES[@]}"; do
  PRUNE_EXPR+=( -path "$p" -prune -o )
done

NAME_EXPR=()
for e in "${EXTS[@]}"; do
  NAME_EXPR+=( -name "*.$e" -o )
done

unset 'NAME_EXPR[${#NAME_EXPR[@]}-1]'

mkdir -p "$DEST"

find . "${PRUNE_EXPR[@]}" -type f \( "${NAME_EXPR[@]}" \) -print0 |
  while IFS= read -r -d '' file; do

    if [[ "$file" == "./scripts/collect_artifacts.sh" ]]; then
      continue
    fi
    if mv --version >/dev/null 2>&1; then
      mv --backup=numbered -- "$file" "$DEST/"
    else
      mv -n -- "$file" "$DEST/" || true
    fi
  done

printf "Moved build artifacts into %s\n" "$DEST"

exit 0
