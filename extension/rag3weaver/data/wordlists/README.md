# Word lists

Two word lists, used by `scripts/french_identifiers.py` to tell an identifier written in French from one written in
English. A word of an identifier counts as French when it has at least four letters, is in `fr/words.json` and not in
`en/words.json` (both compared in lower case and without accents), or when it is in `always_french.txt`.
`never_french.txt` silences a word.

| Folder | Package | Version | License | Words |
|---|---|---|---|---|
| `fr/` | [`an-array-of-french-words`](https://github.com/words/an-array-of-french-words) | 2.0.0 | MIT (`fr/LICENSE.txt`) | 336 524 |
| `en/` | [`an-array-of-english-words`](https://github.com/words/an-array-of-english-words) | 2.0.0 | MIT (`en/LICENSE.txt`) | 274 937 |

Both were taken on 21 November 2026 from the npm registry, as the package tarball alone
(`https://registry.npmjs.org/<package>/-/<package>-2.0.0.tgz`): nothing in it was run, no `npm install`. Only
`index.json` (renamed `words.json`, unchanged) and `license` (renamed `LICENSE.txt`: the push guard reads a bare `LICENSE` as a script that something calls) are kept. The tarballs matched the
registry's own sums:

| Tarball | sha256 | sha512 (registry `integrity`) |
|---|---|---|
| `an-array-of-french-words-2.0.0.tgz` | `037dd0983a4cb18c9d5226b0fdad7943d8f1d562f0a5a5b50b2b989f06f347f2` | `sha512-LRPhVZLteE0wrUFimKWcrEtXZcv8XiG9TArm37j7/PdO2G5f6UfgCny+ibH8CascxMXFMzTPlJcP01PgSmei8Q==` |
| `an-array-of-english-words-2.0.0.tgz` | `24a6f44cd209c8127ad0afacfff935e2af0831e4b0ed5fa62cd0e2c3774e2f2a` | `sha512-FXnNvZSOI27kkKXeLSquhaTGP7z198UOQ4txaYO9fCfrjCh+D5SV7G7XqzEH0229+pAi4cjBEZ4WIQYgjKtO7Q==` |

The English list is broad: it holds many French words of this code (`depot`, `dossier`, `etat`, `tache`, `poser`…).
Those are in `always_french.txt`. On the sixty commits before it, the rule reported nine identifiers, all French.
