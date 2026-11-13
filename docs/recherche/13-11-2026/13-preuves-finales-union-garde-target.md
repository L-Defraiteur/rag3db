# Qualification union de la garde target, preuves finales

APP exact `9ec421069320b2f0cca4099fa27f83059e944de9`, PILOTE exact
`65ad0810ca3f4f1496539d5e2945e5c959b94a07`, sources restées gelées.
Préfixe acquis dans [rapport12](12-preuves-prefixe-union-garde-target.md),
levée de pause runtime C7 distincte après décision publiée68325c713.
Aucun nouveau build, SDK, exclusion ou changement source après le ledger.

## Résultats et comptes

- Reprise réelle commune du target698 vers source9ec VERT, empreintes externes
  conservées, aucune purge manuelle ni attribution fictive.
- F8 canonique8configurations VERT ; compilation large233.9486s/NAV39.0243s,
  inventaire préalable lib2374/66ignorés et NAV40/0ignoré.
- Lib ENTIÈRE SDK29 : **2308passés/0échec/66ignorés**,50.58s de harnais,
  aucun filtre. Toutes les fonctions éligibles de l'inventaire ont leur succès.
  `dataflow::migrations::physical_index::tests::native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes`
  est inclus et `ok`. Le vert SDK29 ne répare pas la CI statique : ticket CI
  reste ouvert, diagnostic doublewriter/ordre de fermeture indépendant.
- NAV : **40passés/0échec/0ignoré**,137.79s. Vrai Chromium observé.
- Deux réponses HTTP réelles app.js/app.css comparées aux blobs Git de APP9ec,
  HTTP200 ;500544/44770octets. Un seul juge lib produit ces deux témoins ;
  ne pas les compter comme deux nouvelles fonctions ni comme deux NAV.
-223gabarits Git/disque/include_bytes/ELF attestés avant ledger puis après
  runtime pour chacun des deux ELF ; originaux et copies rehashés identiques.

Les suites CPU10/toy8/journaux7/builtin9 acquises séparément ne sont pas relancées
isolément. Les fonctions présentes dans la lib entière sont des régressions,
pas de nouveaux juges comptés plusieurs fois. Aucun lot physique ancien,
PG/BGE ou scénario de service partagé qualifié par cette passe.

## Provenance des binaires et principal réel

Ledger immuable avant runtime SHA
`6937bd93b2e87fe9cd752b87eb5e2b1c280d9afa83655a9fd278fe5c1faa29b7`,
snapshot100fichiers d'avant runtime conservé sur deux postes, aucun remplacement.
ELF large467285648octets SHA
`f28a3f993af3b7d45f79d40ff94d6b86e928c60a65fe3973123ccb605d4664b3`,
closure13 avec default ; NAV467193136octets SHA
`71d41f8b8c9a675e4c40757e5aaf40f96eebe6a060092464c72320455140c266`,
closure14 avec default/navigateur. Listes exactes dans le ledger publié12.
Aucune confusion avec les anciens ELF CPUfeatures[].

SDK29 `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
SHA `b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929`.
Même inferior libPID1232946/start91647819 : exec/main SDK7champs stricts,
raw nonvide/diff{}, exeSHA et cgroup exacts, maps SDK29 au principal réel.
Capture main916478.92568723, detachTracerPid0 à916478.929399872,
puis exécution horsptrace ; vrai parentPID1232943 waitstatus0/exit0
à916529.586395163. Aucun code de retour deviné de GDB ne sert de résultat.
NAV corrélé au deuxième ELF, profil SDK réel et maps relevés.
Pool1GiB et quatre rôlesCPU fixés ; états privés frères xdg-state/state-fixture.

## Budget, ressources et observations

START ORIGINAL914593.915525602, END ORIGINAL919393.915525602 ; reçu de reste
après poste et END EFFECTIF919390.748839204. Finaliseur composé VERT à
**2076.554637075s depuis STARToriginal <4800**, incluant rouge d'entrée,
préparation de reprise et pause C7. Aucun reset de fenêtre.

Scope validation-v57s3cvy VERT0/restes[]/nettoye=true, autoritéPID1125874
start91520369 TERMINE. Bus `/tmp/r3v-aae67739` absent, launcher1125780/poste1125842/
autorité1125874 absents. Quiescence finaliseur[] puis scan indépendant frais
exe/cwd/FD/owned[] avec autoritéterminée et busabsent.
Pic mémoire15069622272octets, événements high/max/oom/oom_kill tous0.
Target6482859945octets conservé. Aucun travail de qualification encore actif ;
poste libéré, aucune suppression de ressources demandée ni effectuée.

Cinq observations deviceFD dans les descendants (voir devices.json),
UNKNOWNowned0 ; sept protections inconnues de base dans le finaliseur,
neuf au scan frais indépendant après la passe (non attribuées).
Ni absenceGPU ni usage de calculGPU inférés de ces FD/maps/compteurs.

## Archives et revue demandée

**558fichiers** size/SHA comparés entre copie locale et preuves distantes,
zéro écart ; manifest SHA
`cece3588719044bf461eb6f7f322eb8f9b0db524e2b63e979eb07f782522cb3d`.
Copie locale horsW `~/.cache/rag3weaver-build/cx-preuves-garde-target-union-v2-final/`,
preuves distantes P/Q/Sv2 et journal/quiescence sous journaux-cx-recherche-garde-target-13nov.
Manifest déposé sur les deux postes. Preuves du préfixe figé100fichiers intactes.

Anciens rouges conservés sans réécriture : ciblé698/v1 (profil d'état erroné,
module8/1), prépa03896 (harness3OK/1ERROR), préfixe union4d38 refus argv-B
avant Cargo (diagnostic8d473,37fichiers). Les corrections ciblées et leurs
preuves demeurent séparées ; le présent rapport ne transforme aucun ancien
rouge en vert et n'attribue pas les rouges CI statiques à ce profil SDK29.

Soumis RO preuves Mémoire/ED/C7 et relecture Recherche. Ticket garde reste
OUVERT, aucun GO master/retrait/service déduit. Une future intégration nécessite
candidat exact, comparaison produit/entrants puis tour master C7 distinct.
