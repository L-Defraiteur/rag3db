# L'enveloppe CPU des lourds (14 novembre 2026)

Demande de Lucie, relayée par c7 : luciepc a tourné à 33 de charge sur 24 cœurs à 15 h, avec quatre lourds (96,
embarquement, lucivy, le cœur), et deux rouges d'environnement en ont découlé (le crash physique, le délai de la
carte de progression). Le but est de **borner sans forcément baisser le nombre de lourds**.

## La mesure (15 h 03 – 15 h 13)

`docs/optimiseur/scripts/mesure-lourds.sh 600` relève toutes les 10 s, pour chaque portée `run-*.scope` que
crée `poste lourd`, le CPU sur l'intervalle (par `cpu.stat`, en % d'un cœur), les tâches (`pids.current`) et la
mémoire (`memory.current`). Le relevé est dans `~/.cache/optimiseur-notes/mesure-lourds-14nov.tsv` aux deux
postes.

**luciepc** (24 cœurs) : charge de 4,3 à 16,4 ; la somme des portées monte au plus à 1346 %.

| lourd | CPU moyen / max | tâches max | mémoire max |
|---|---|---|---|
| check_features de lucivy-437-2 | 412 % / 802 % | 69 | 6,6 Gio |
| run_e2e de 96 (p1r-96) | 248 % / 385 % | 166 | 5,8 Gio |
| cargo_magasin test de lucivy | 238 % / 517 % | 26 | 3,2 Gio |
| reprise/passe.sh | 146 % / 474 % | 783 | 15,2 Gio |
| concurrence_test du cœur | 143 % / 298 % | 55 | 7,9 Gio |
| sonde-bati d'ED | 100 % / 100 % | 12 | 2,5 Gio |

**Portable** (32 cœurs, `lourds-max` 1) : charge de 3,5 à 20,5. Trois portées tournent en même temps malgré une
seule place : les sous-portées des confineurs et des passes composées ne passent pas par la file.

| lourd | CPU moyen / max | tâches max | mémoire max |
|---|---|---|---|
| wt-verrou-passe.sh (la passe de l'IDE) | 716 % / **2852 %** | 186 | 9,3 Gio |
| deux-passes.sh | 544 % / 800 % | 145 | 16,2 Gio |
| wt-1b-copie.sh (arbre principal) | 208 % / 1593 % | 117 | **32 Gio, son plafond** |

Aucun lourd n'est seul en cause : ce sont les pointes qui s'additionnent, une passe montant à 8 ou 28 cœurs
pendant que les autres tournent.

## L'enveloppe (`extension/rag3weaver/scripts/poste`)

- **Cœurs utiles** = cœurs du poste − `coeurs-libres`. `coeurs-libres` est un fichier d'une ligne du dossier des
  verrous (`~/.cache/rag3weaver-build/`), à côté de `lourds-max` ; il vaut 2 s'il manque, et
  `POSTE_COEURS_LIBRES` l'emporte.
- **Un lourd** reçoit utiles / `lourds-max` cœurs (division entière, au moins 1) ; **une mesure** les reçoit tous.
  La portée `systemd-run --user --scope` porte `CPUQuota` = cœurs × 100 %, à côté de `MemoryMax`.
  `POSTE_CPU_QUOTA` donne un autre quota pour une commande, en le sachant.
- **Les fils dits aux outils** : `POSTE_FILS` (la part), et `RAYON_NUM_THREADS`, `CARGO_BUILD_JOBS`,
  `RUST_TEST_THREADS`, ramenés à la part s'ils sont absents ou plus grands, sont exportés dans l'enveloppe.
  Un poste sous un poste hérite de l'enveloppe de son ancêtre.
- Ce que cela donne : à luciepc (24 cœurs, 4 lourds), 22 / 4 = **5 cœurs, 500 %** par lourd ; au portable
  (32 cœurs, 1 lourd), 30 cœurs.
- **La borne est le quota.** Un script qui pose lui-même `CARGO_BUILD_JOBS=8` ou `-j 14` après `poste` n'est pas
  ramené, mais il reste dans son quota. Une sous-portée créée hors de `poste` n'est bornée que par la sienne.

## Le témoin (`extension/rag3weaver/scripts/test_poste.sh`, cas 8)

Quatre places et quatre cœurs utiles (`coeurs-libres` = cœurs − 4). Chacun des quatre lourds lance huit
boucles actives pendant 4 s, puis on lit le temps CPU de sa portée.
- Avec l'enveloppe : **16 386 ms** de CPU en 4 s, pour une borne de 18 400 ms (4 cœurs × 4 s, à 15 % près).
  Les fils lus valent 1.
- Avec le `poste` de master : **66 448 ms**, rouge.
- Le cas 6c (« un tenant qui reçoit un signal garde le verrou ») est rouge sur master comme sur cette
  branche : il préexiste et n'est pas de ce lot.

## Posé avec le lot

- `coeurs-libres` explicite, à **2**, dans `~/.cache/rag3weaver-build/` aux deux postes (mot de c7). `lourds-max`
  reste à 4 à luciepc et à 1 au portable.
- `poste` recopié de master dans `~/.cache/rag3weaver-build/poste` aux deux postes, après le push.

## Deux tickets ouverts à côté

- `docs/optimiseur/tickets/2026-11-14-un-tenant-qui-recoit-un-signal-ne-garde-pas-le-verrou.md` : le cas 6c
  du témoin, rouge sur master.
- `docs/optimiseur/tickets/2026-11-14-les-sous-portees-ne-passent-pas-par-la-file-des-lourds.md` : trois portées
  pour une place au portable.
