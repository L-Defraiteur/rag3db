# Juger F300 et N rejouée de la mesure longue de l'archiviste

- **État** : ouvert.
- **Famille** : produit
- **Gravité** : moyenne. La comparaison des variantes de l'archiviste repose sur une référence N jouée
  sur un ancien binaire, avec des pannes du 7879. La N rejouée lève cette réserve, mais elle n'a pas
  été lue.
- **Atteignable en service** : oui. L'archiviste range les sessions longues.
- **Touche rag3weaver** : non. Une mesure.
- **Pour** : une session Codex (passation du 16 novembre).
- **Signalé par** : banc, 16 novembre.
- **Ouvert le** : 16 novembre 2026

## Le constat

La page `docs/banc/15-11-2026/02-jusqu-ou-l-archiviste-voit.md` s'arrête au § 9 (F à 50 k, 25 %).
Deux passes ont fini sans être jugées. Elles sont à luciepc, sous
`~/.cache/rag3db-banc-notes/compaction/long/` :
- `F300-ov10-0653-20261116-0157` : 244 tours ; décisions 38 `create`, 140 `fiche`, 21 `none`, et
  **45 sans `kind`**. Le journal (`../long-reprise.log`) finit sur des tours de 0,4 s qui rendent
  `[(None, None, '')]`, avec 45 « sections sans compteurs ». Le contexte y dépasse 500 000
  caractères ;
- `ref-N-0653-20261116-0305` : 244 tours ; décisions 54 `create`, 141 `fiche`, 49 `none`.

Deux variantes sont déjà jugées contre l'ancienne N, mais pas encore écrites dans la page : F 25 %
variante (P1 0,69) et S 50 (P1 0,77). Les juges sont `long/juge-*.json` et `long/rien-*.json`.

## Ce qu'il faut faire

1. **Les 45 décisions vides de F300.** Lire `F300-…/appels.jsonl` (une ligne par tour : `reponse`,
   `s`, l'usage) et `F300-…/backend.stderr`. Il faut dire si c'est un refus du modèle (taille de
   l'invite, quota), une erreur du backend, ou une section sans décision voulue. Un refus de quota
   se lit dans les champs d'erreur, jamais dans le texte archivé : le 15 novembre, un « 429 » trouvé
   dans le texte a été pris à tort pour un quota.
2. **Rejuger contre la N rejouée** : F 25 %, F 25 % variante, S 50 et F300, par `j_sujets.py` puis
   `r_rien.py`, sur le modèle de `~/.cache/rag3db-banc-notes/compaction/juge-f25.sh`. Remplacer `N`
   par `long/ref-N-0653-20261116-0305` explicitement : le `ls | tail -1` du script prend le dernier
   par ordre alphabétique. Le juge appelle Vertex : appels en série, attente doublée à partir de
   60 s sur un 429 ou un 503. Les identifiants s'utilisent par leur chemin seulement
   (`GOOGLE_APPLICATION_CREDENTIALS=$HOME/git_workspaces/rag3db/.vault/vertex-sa.json`) : `.vault`
   ne se lit jamais en clair.
3. **Écrire le § 10** de la page : une table par variante (décisions, fiches, P1, P2, « rien » qui
   endort, s par décision, jetons d'invite et part du cache), contre la N rejouée, et ce qu'elle
   change aux § 6 à 9.

## La preuve

- Chaque juge dit son nombre de décisions (244). Une passe qui n'en a pas 244 n'est pas jugée.
- Le N contre N (`long/juge-N-contre-N.json`) sert de témoin : rejugé contre lui-même, il doit
  rendre P1 = 1.

## Taille

Trois à quatre heures, surtout des appels Vertex en série.
