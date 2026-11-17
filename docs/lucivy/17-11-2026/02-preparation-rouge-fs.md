# Source RED FS et recette proposée — non exécutée

Code poussé `8b9435363dbd8ec0e3973f2368b641189f0726eb`, parent1ee5.
[Ticket ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Préparation autorisée par C7 le17nov. **Aucun Cargo, rustfmt, ELF ni test joué.**
Aucun contact corpus réel, même métadonnées. Seul le lock source du worktree PC
a été lu pour auditer SHA2 ; les pins sont [ici](03-fs-red-source-pins.json).

## Raccord livré, défauts préservés

`common/fs_fixture.rs` porte CapturePlan, StdFs, CaptureFs, Snapshot avec chaînes
UTF8 privées et accès immuable, pins ordinal/chemin/taille/SHA256, panel15 partagé,
Trace et ManifestSink. Politique `v3-contains/fs-effective-order-v1`, cap5000,
exclusions historiques. Pas de tri ; chaque répertoire est énuméré une fois.
Le vecteur des entrées est matérialisé avant leur traitement ; l'ordre renvoyé
par StdFs reste effectif, sans garantie de stabilité entre captures différentes.

Le vrai `v3_ground_truth_contains` appelle `with_v3_contains`, puis son vrai
builder, report et run_panel dans la continuation. Les autres diagnostics gardent
leur ancienne collecte. Le panel15 a été déplacé sans changement de valeurs ni
modes. Les personnalisations d'environnement restent actives dans cette baseline.
Le garde historique racine absente est conservé avant l'entrée ; il devra passer
dans l'admission lors du correctif, après vérification des flags.

Les erreurs enumerate/metadata/read sont encore avalées ; UTF8 invalide est
encore éliminé ; vue vide retourne Ok(None) ; aucune couverture n'est contrôlée.
La création d'index native peut toujours réutiliser/supprimer/persister : Trace
observe désormais les sites réels avant read_shape/remove_dir_all/persist_unsynced.
Ce ne sont pas encore des refus de ces effets. L'oracle natif relit encore les
chemins : **aucune qualification read-once native**.

ManifestSink est un transport préparé : create_new exclusif, SHA256 du corps JSON,
write_all/sync_all puis fsync parent. Construction sans I/O. La baseline ne
l'appelle PAS : publier ADMITTED ici mentirait. Le futur correctif doit écrire
ADMITTED ou REFUSED précis avant le builder et rendre l'échec du sink bloquant.
Le format de rejeu est proposé, son consommateur n'est pas implémenté.

SHA2 est une dev-dependency exacte `=0.11.0`, déjà dans le lock PC audité avec
digest0.11.3. Aucun code cryptographique maison. Le lock n'est pas suivi par ce
dépôt. Sa modification minimale hors ligne pour la dépendance directe devra
être archivée ; aucun Cargo de résolution n'a été lancé pendant cette préparation.

## Témoins proposés, aucun résultat observé

Tous traversent la même entrée de collecte que le natif, avec petits fichiers
possédés sous TempDir et SpyFs déléguant à StdFs. Les erreurs sont injectées à
l'opération/chemin précis, sans dépendre de chmod ni des privilèges du poste.

| Témoin exact | Faute actuelle attendue |
|---|---|
| empty_view_is_refused_before_builder | Ok(None), aucune preuve REFUSED |
| enumeration_error_is_not_an_empty_subtree | sous-répertoire illisible ignoré, builder appelé |
| metadata_error_is_not_a_filtered_file | candidat ignoré, builder appelé |
| read_error_is_not_a_filtered_file | candidat ignoré, builder appelé |
| invalid_utf8_is_refused_not_dropped | candidat invalide ignoré, builder appelé |
| uncovered_panel_is_refused_before_zero_equals_zero | vue nonvide sans les15 termes, builder appelé |
| covered_fixture_reaches_builder_once_on_the_captured_bytes | positif : tous15 couverts, builder1/read1 |

Les six négatifs exigent le motif exact, builder/report/reuse/delete/persist0
et une preuve REFUSED. La première assertion échouera probablement sur le motif ;
les traces sont imprimées avant. Le positif évite le faux correctif reject-all.
Le builder est une continuation espion dans ces témoins : aucun vrai index ni
search natif n'est joué. Le positif ne revendique ni comparaison index/oracle,
ni publication ADMITTED, ni qualification du natif réel. Les fautes de chaque
entrée read_dir ne sont pas encore un témoin distinct de l'échec du répertoire.

## Recette courte proposée, RO puis GO distinct avant Cargo

Ce texte n'est **pas** un pilote exécuté ni un GO. Avant lancement, préparer Q
privé neuf hors worktree/target, nonce et marqueurs, pilote/admission/préflight
épinglés et relus. Une préparation PC ne touche aucun corpus. Source exacte
8b94353 transférée dans un worktree dédié, état propre, identité et copies contrôlés.
Même unique T3552, provenance connue inchangée ; aucun target frère/purge/import.
Archiver les trois anciens ELF qualifiés en lecture seule avant remplacement,
sans les rejouer. Aucun contact ELF avant le GO spécifique.

Un seul poste lourd, timeout dur **300s** (TERM295 + kill5), Common285,
32G mémoire max/high max, swap0, quotaCPU4 et jobs4 au plus, deux cœurs libres.
Le budget inclut préflight, compilation, inventaires, tests, preuves et quiescence.
Contrôler la disponibilité réelle ; aucun run natif/corpus A/B.

Commande de compilation prévue, une seule, hors ligne :

```text
cargo test --offline -j4 -p lucivy-core --test test_fs_fixture_admission
  --test test_sfx_v3_ground_truth --no-run --message-format=json
```

T explicite via CARGO_TARGET_DIR, aucun ABSENT_OK ni V3_* hérité, TMPDIR sous Q.
Copier le lock audité seulement après contrôle SHA ; --offline refuse un cache
manquant, sans téléchargement ni substitution de version. Archiver lock avant/
après et vérifier sha2/digest inchangés. CARGO_TARGET_TMPDIR des reports reste
celui compilé par Cargo, pas TMPDIR : aucun report natif n'est exécuté ici.

À partir des messages compiler-artifact : écrire et fsync le **ledger des deux
ELF avant toute invocation** (target, chemin réel, SHA, source, commande), puis
inventorier les deux --list et publier l'inventaire avant les juges. Exiger les
sept noms exacts sans ignores, et présence du vrai v3_ground_truth_contains dans
l'autre ELF ; aucun corps natif lancé. Jouer chaque juge exact --nocapture,
threads1, budget restant commun. Continuer après les rouges attendus pour
observer les six causes, pas après une erreur d'infrastructure/délai/provenance.
Attendu non observé : six échecs sémantiques, positif1pass, aucun skip.
Un échec de compilation ou infra n'est pas le rouge d'admission demandé.

Après : rehash physique des deux ELF, source/lock/copies/provenance, scratchs,
preuves partielles publiées même sur erreur ; verdict Common terminé/restes[]/
nettoyé et absence fraîche des PID/start propres avant restitution réelle à C7
et Recherche. UNKNOWN nonattribués conservés et nommés, aucune quiétude globale
inventée. Archiver les journaux et SHA ; ne pas corriger la gate avant que les
quatre familles (avec les trois sous-cas IO annoncés) aient réellement rougi.

Prochaine étape : RO statique de la source et de cette recette. Ensuite préparer
le pilote concret selon les remarques, le faire relire et attendre le GO distinct.
