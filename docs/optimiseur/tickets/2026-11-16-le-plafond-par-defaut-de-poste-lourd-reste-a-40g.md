# Le plafond par défaut de `poste lourd` reste à 40G, au-dessus des 32G des validations

- **État** : ouvert (16 novembre 2026) — attend deux relevés, puis le mot de c7
- **Famille** : produit (outillage : `extension/rag3weaver/scripts/poste`)
- **Gravité** : gêne. Depuis 627d8385f6, une validation sous plus de 32G est refusée avant la file, et le refus se
  nomme. Mais chaque session doit encore penser à `POSTE_MEM_MAX=32G`.
- **Atteignable en service** : non
- **Touche rag3weaver** : oui (`scripts/poste`, copié au cache des deux postes)
- **Pour** : la session qui reprend l'outillage de G (Codex)
- **Signalé par** : hotfixs (15 novembre), décision de c7 (16 novembre : refus avant la file d'abord, le défaut après mesure)

## Ce qu'on sait

- `scripts/poste` : `MEM="${POSTE_MEM_MAX:-40G}"`. `VALIDATION_MEMORY_MAX=34359738368`, soit 32 Gio, le
  `MAX_MEMOIRE` de `scripts/confiner_validation.py`, comparé par `test_poste.sh`.
- Premier relevé du cœur (16 nov.) : 8,1 Gio de `memory.peak` pour `build/tsan`, -j8, cibles `concurrence_test` et
  `rag3db_vector_extension`. Ce bâti a été servi en partie par le cache de compilation.
- Manquent : le pic d'un bâti **à froid** de `build/moteur` (Release, `e2e_test` et `transaction_test`), que le cœur
  relève au prochain bâti qui ne tombe pas dans le cache, et le pic d'un bâti de lib du magasin
  (`docs/procedures/scripts/batir-lib.sh`, `cmake --build -j 8` sous `poste lourd`) **à froid** (effacer
  `~/.cache/rag3weaver-build/wt-lib-arbre/build/lib` avant, avec le mot de c7).

## Ce qu'il faudrait

1. Relever le pic par `memory.peak` de la portée de `poste` : `cat /sys/fs/cgroup<cgroup du ninja>/../memory.peak`,
   ou le cgroup du scope `run-p….scope` lu dans `/proc/<pid du cmake>/cgroup`. La dernière valeur lue avant la fin est le
   pic. Noter le chiffre dans ce ticket, avec la commande et le poste.
2. Si les deux pics tiennent largement sous 32 Gio (moins de 24), le défaut de `poste` passe à 32G, avec une seule source
   (`VALIDATION_MEMORY_MAX`, ou une variable lue des deux côtés). L'en-tête de `poste` le dit, et `test_poste.sh`
   vérifie qu'un lourd sans `POSTE_MEM_MAX` reçoit 32G. Sinon, le défaut reste 40G, et ce ticket se ferme sur les chiffres.
3. Après l'atterrissage, recopier `poste` au cache des deux postes (forme dans la page de passation de G).
