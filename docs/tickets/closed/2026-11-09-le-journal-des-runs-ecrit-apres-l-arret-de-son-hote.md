# Le journal des runs écrit encore après l'arrêt de son hôte

- **État** : corrigé (13 novembre 2026, lot journal-apres-arret de la session recherche) : l'arrêt de l'hôte écrit les journaux en attente de leur grâce, attend leurs fils, et plus rien ne s'écrit après.
- **Famille** : produit
- **Gravité** : confort pour les tests (un TempDir à moitié effacé dans /tmp, qui est de la mémoire
  vive) ; à départager en service (un hôte arrêté dont un run finit d'écrire son journal).
- **Atteignable en service** : à départager.
- **Touche rag3weaver** : oui, `daemon_host/journal_des_runs.rs` et l'arrêt de `DaemonHost`.
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

## Ce que c'est

Le témoin de suite (run_e2e.sh, la liste des résidus) a vu `e2e_juge` laisser un TempDir qui garde
`runs/<backend>/graph-*.json.zst` et `ingest-*.json.zst` : des journaux de runs écrits après la fin
du test. Passe du lot arret-du-backend, 9 novembre 2026, 13 h 22 (luciepc) :

```
▸ RÉSIDUS e2e_juge : 1 entrée(s) dans son TMPDIR (20K)
    20K	.tmput0P1O
      24 o	runs
      262 o	runs/d49c783a84a7
      551 o	runs/d49c783a84a7/graph-1a1c042c50c-23.json.zst
      363 o	runs/d49c783a84a7/graph-1a1c042c48b-20.json.zst
      370 o	runs/d49c783a84a7/ingest-1b6194402957-1794226963793-3.json.zst
```

## La cause (lue dans le code)

`JournalDesRuns::noter` écrit le journal d'un run racine après une grâce : un fil détaché
`journal-ecrire` dort `GRACE` (250 ms) puis relève le journal par un `Weak` et l'écrit
(journal_des_runs.rs, `noter`). Ce fil n'est ni inscrit ni attendu : l'arrêt de l'hôte ne l'attend
pas. Un test qui lâche son hôte puis son TempDir (`let d = tempdir(); let (host, …) = banc(…)`)
laisse un fil qui se réveille pendant l'effacement, relève encore le journal (un autre détenteur le
tient) et écrit dans un dossier qui part.

C'est la famille du ticket fermé `closed/2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md`
(un fil de fond non attendu à l'arrêt), au niveau de l'hôte et non du backend.

## Ce qui reste

L'arrêt de l'hôte (`shutdown`, et son Drop) écrit tout de suite les journaux en attente de leur
grâce et attend ses fils d'écriture ; un `TempDir` qui part après l'hôte ne voit plus rien venir.
Témoin : un hôte de test lâché juste après la fin d'un run ne laisse rien écrire après son Drop.
e2e_juge sort alors de la liste des résidus, prouvé par plusieurs passes propres.

## Le correctif (13 novembre 2026, lot journal-apres-arret)

- `JournalDesRuns` garde les runs racines finis qui attendent leur grâce, et les poignées de leurs
  fils `journal-ecrire`. `vider()` marque le journal fermé (un événement arrivé ensuite n'écrit plus
  rien), écrit tout de suite les journaux en attente, puis attend les fils ; un fil qui se réveille
  après ne réécrit pas un run déjà écrit. Un fil d'écriture qui ne part pas écrit tout de suite.
- `DaemonHost::shutdown()` appelle `vider()` après la fermeture des backends ; un `Drop` de
  `DaemonHost` aussi (un hôte de témoin lâché sans `shutdown()`, comme e2e_juge).
- Témoins (`src/daemon_host.rs`) : `l_arret_de_l_hote_ecrit_les_journaux_en_attente_et_rien_apres`
  et `un_hote_qu_on_laisse_tomber_ecrit_les_journaux_en_attente_et_rien_apres`, rouges sur le
  code d'avant (« au retour de shutdown() / du Drop, le journal du run fini juste avant est
  écrit »), verts sur le correctif (0,9 s).
- e2e_juge sort de la liste des résidus (`tests/e2e/suites_qui_laissent_des_residus.txt`),
  prouvé par la passe du lot et trois rejeux d'e2e_juge propres.
