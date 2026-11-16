# Rouge de la qualification des scripts de fixture

APP **734087a4e99d54331aac2f6321df9ef6da3b6d9c**, PILOTE
**38304227d75e878cceb8ce4f08e2d246e7835bf9**, GO exact C7. Source et pilote
figés, base master ties d395. [Ticket ouvert](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md).
Aucun changement produit ni nouveau pilote après ce rouge.

## Résultats réellement acquis

- Témoin canonique `codex_threads` : **6/0/0**,0,31s,2662filtrés. Le nouveau
  helper et les deux fixtures historiques sont donc compilés et joués ici.
- F8 canonique : **8/8**, et preuve `rag3weaver revérifié` acquise.
- Lib entière large : **ARRÊT au premier FAILED**, pas de résumé final ni total
  de juges qualifiés. NAV et seconde lib **NON JOUÉES**. Aucun VERT global.

Premier échec :
`pont_navigateur::temoins_relai::l_ecoute_range_un_pont_par_navigateur_et_le_socket_est_a_soi`.
Deux autres FAILED relai sont déjà dans le buffer de cette observation (second
hôte et remplacement de connexion) ; ce ne sont pas des relances. Le watcher
qualifié lit le vrai `official/lib.log`, inode164595051/device55,216530octets,
SHA **e8613835400aefdd639ef4641c15b567e1fda6d46caf2ff6e41c1a152f489220**.
Le préfixe exact de216530octets est conservé et son SHA revérifié ; le journal
complet a pu recevoir des lignes pendant l'arrêt. `first-red.json` immuable,
observé1180523.770781667, soit948,092207574s depuis START.

## Portée et conservation

INITIAL1179575.678574093, END1186775.678574093,7200uniques après vrai poste.
Plafonds effectifs32GiB/HIGHmax/swap0, cpu.max500000/100000, quota cinq cœurs
par place. Pin59ac et SO b7e9...2101/vector1669...7561, magasin explicite, canon2150.
Profils sans jeton conservés. Les statistiques GPU/swap du rapport de charge
sont globales à la machine ; aucune attribution ni absence GPU déduite.
Pic mémoire observé en cours9951186944octets/events0, **pas le pic final** :
le cgroup avait déjà disparu au relevé final, aucune valeur finale inventée.

Scopevalidation-9ilxlu_h **ROUGEcode1**, erreur « descendants laissés après la
commande », restes[],nettoyé=true. Retour extérieur1, verdict composéRED.
Le premier FAILED a fait quitter le driver ; le confineur a arrêté les descendants.
Contrôle indépendant1180595.669686168 : source/target exe/cwd/FD found[],
toutes identités PID/start attribuées absentes, owned_unknown[], autoritéTERMINE,
bus et cgroup absents. Douze UNKNOWNglobaux nonattribués archivés sans blanchiment.
Poste rendu ; target5087074359octets conservé, sans purge ni reprise.

ELF réel de lib archivé APRÈSrouge,484292712octets, SHA
**6f12273abeb4a957dd5abdcd0f518ee673607146eefbe2c2938769d9bfdf8e74**,
original et copie hachés identiques. Ce ledger postrouge n'est pas une attestation
publiée avant les tests. Le ledger prévu avant deuxième lib n'a pas été atteint.

**26fichiers** archivés PC puis copiés portable, SHA+taille local26/26 sansécart.
[Manifeste](script-fixtures-red-v1-manifest.json) SHA256
**904832a0385f0e3b75c8e588f564094dc33a003844a35707e0d4e3499b955029**.
Racines `~/.cache/rag3weaver-build/cx-proofs-memoire-script-fixtures-red-v1`
et suffixe`-local`,{client,controller,supervisor}. Autorité privée/jeton exclus.
Premier rouge, ancien pilote1e3 retiré et historiques système conservés.

## Diagnostic RO avant toute correction

Les répertoires réels `tmp/pont-986542-N` sont encore présents. TMPDIR du pilote
mesure84octets ; `pont-986542-0/pont.sock` donne108octets et sa variante
`ecoute/pont.sock`115octets. `dossier()` dans témoins_relai.rs compose ces chemins
sur std::env::temp_dir ; sa propre doc rappelle la limite108octets du socket Unix.
Cela fournit une cause de profil plausible et précise : chemin trop long.
Le STOP avant fin de lib n'a pas livré le détail du panic capturé ; on ne prétend
pas avoir lu un errno particulier dans cette passe. Aucun témoin produit rejoué.

Proposition seulement à C7 : un TMP privé court, sous mêmeENDoriginal avec
reste effectif après poste et nouvelle RO/GOexacte, en conservant P/Q/Sv1 et T.
Cette variante nécessiterait une admission de cache porté documentée et strictement
vérifiée ; pas de copie d'ELF inventée, pas de reset7200, pas de relance automatique.
Ticket reste ouvert, aucune intégration/retrait/master déduit.

### Complément diagnostic RO, sans rejeu

Le [ticket fils socket](../tickets/2026-11-16-le-profil-tmp-prive-depasse-la-longueur-des-sockets-des-temoins-du-pont.md)
porte cette suite distincte. Snapshot supplémentaire PC/local byteidentique
`client/socket-diagnostic.json`, SHA93cabda18895dc12031a0eef061a86c2834a2c062817789a2c3d6a2f94641c7a :
7dossiers réels, UID/modes/inodes/absence socket, géométrie108/115octets,
profil HOME/XDG/TMP/DBUS/états, quotas relevés, Nodev22.22.3 (commande version
seule, rc0). Les26entrées du premier manifeste restent inchangées ; ce supplément
est séparé et ne repeint pas leur rouge. Aucun témoin produit/service rejoué.
C7 impose HOLD : aucun helper/profil modifié jusqu'au diagnostic et à sa RO.
