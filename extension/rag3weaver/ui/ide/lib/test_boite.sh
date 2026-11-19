#!/bin/bash
# The witness of lib/boite.ts (mode pair, lot 4): node reads the TypeScript without a bundler.
set -e
cd "$(dirname "$0")"
node --experimental-strip-types --no-warnings boite.test.mjs
