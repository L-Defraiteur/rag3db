#!/bin/bash
# The suites a range of commits requires (required_suites.txt, next to this script, or a given table), one line per
# suite: "<suite> <first path of the range that requires it>". Nothing when no path of the range matches.
# Usage: required_suites.sh <base> <head> [table]   (run inside the repository)
set -u -f
base=$1; head=$2; table=${3:-$(dirname "$(realpath "$0")")/required_suites.txt}
[ -f "$table" ] || exit 0
files=$(git diff --name-only "$base" "$head") || exit 2
while read -r suite globs; do
  case "$suite" in ''|'#'*) continue ;; esac
  for f in $files; do
    for g in $globs; do
      case "$f" in $g) echo "$suite $f"; continue 3 ;; esac
    done
  done
done < "$table"
