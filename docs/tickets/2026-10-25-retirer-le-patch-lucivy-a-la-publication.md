# Retirer le patch lucivy à la publication

- **État** : ouvert, le 25 octobre 2026 (G, sur décision de Lucie relayée par l'orchestration).
  rag3weaver prend lucivy par un `[patch.crates-io]` **temporaire** : la branche
  `correctifs-rag3db-19-octobre` du dépôt https://github.com/L-Defraiteur/lucivy, épinglée à `bb07627`.
  Il en sera ainsi jusqu'à ce que Lucie approuve cette branche sur main et publie une version.
- **Gravité** : aucune en service. C'est une dette de construction : rag3weaver ne prend plus lucivy
  par sa version publiée, ce qui est notre règle.
- **Atteignable en service** : sans objet.
- **Touche rag3weaver** : oui (`extension/rag3weaver/Cargo.toml`, `Cargo.lock`).

## Ce que c'est

La version publiée de lucivy-core (4.3.0) a deux défauts que rag3weaver ne peut pas contourner seul :

- `2026-10-19-le-plein-texte-lucivy-se-fige-dans-son-commit-pendant-une-reindexation-lue.md` : le
  commit d'un shard se fige sous des recherches pendant une réindexation (interblocage par
  réentrance). Correctif `dbb7d6e`.
- `2026-10-19-un-shard-de-lucivy-sans-ecrivain-perd-les-ecritures-en-silence.md` : une écriture
  vers un shard sans écrivain est jetée avec `Ok`. Correctif `bb07627`.

Lucie ne publie pas maintenant. La branche sert de branche temporaire, et elle l'approuvera sur
main plus tard, dans une session lucivy. D'ici là, le patch porte les deux correctifs dans rag3weaver.

## Ses filets

- `extension/rag3weaver/Cargo.toml`, bloc `[patch.crates-io]` : les quatorze crates du dépôt lucivy
  qui sont dans le lock (ld-lucivy, lucivy-core, luciole, lucistore, sparse-vector, lucivy-fst et les
  ld-lucivy-* / ld-ownedbytes en 0.27.0), tous pris à `bb07627`. Ils sont patchés ensemble : un seul
  crate patché tirerait ses voisins du même dépôt en double de ceux du registre.
- `extension/rag3weaver/Cargo.lock` : ces quatorze crates pointent vers le dépôt git.

## Le témoin

Le témoin réel `e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot` (219 fichiers, quatre
lecteurs, `RAG3WEAVER_MESURE_ARBRE`) se fige au tour 1 sans le correctif. Avec lui, il passe ses
trois tours. Voir le ticket du figement.

## Pour le fermer

Quand Lucie publie la version de lucivy qui porte les deux correctifs :
1. retirer les quatorze lignes du `[patch.crates-io]` (et le commentaire qui les présente) ;
2. monter `lucivy-core`, `ld-lucivy` et `sparse-vector` à cette version dans `Cargo.toml`, puis
   `cargo update` sur les quatorze crates ;
3. vérifier que `Cargo.lock` ne contient plus de source `git+https://github.com/L-Defraiteur/lucivy` ;
4. fermer en même temps ce ticket et les deux tickets lucivy, puis jouer la passe complète.
