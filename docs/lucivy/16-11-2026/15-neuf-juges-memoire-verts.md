# Neuf juges mémoire verts ; poste rendu, qualification bornée

Ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
GO exact **C7-LUCIVY-GREEN-6C701-16nov300**, recette14,
APP1ee5b366ca67d7fe6926da232a6650d1b1d81407/pilote6c701/admit22550/préflight277a,
Qgreenv1nonce023317833614261f17091d13c8a4988a. Une passe, zéro retry.

## Résultat réellement exécuté

Compilation offline release des trois targets réussie **3,37s**. Ledger3ELF
persisté avant tout inventaire, ledger des trois inventaires persisté avant
les juges. Les neuf fonctions mémoire sont ensuite jouées chacune par --exact,
un thread, **1passé/0échec/0ignoré**, soit **9/0/0**. Chaque appel filtre les huit
autres noms ; à l’échelle de la passe les neuf ont bien été exécutés. Ce ne
sont pas huit ignorés transformés en verts.

[Logs individuels et traces](preuves/derived-admission-green-1ee5/SHA256SUMS),
[résultat du pilote](preuves/derived-admission-green-1ee5/logs/result.json).

| Juge mémoire | Résultat |
| --- | --- |
| missing_panel_is_rejected_before_any_builder | 1/0/0 ; refus précis du négatif50 puis passage unique du positif distinct |
| loader_error_is_not_an_empty_corpus_or_no_hit | 1/0/0 |
| invalid_utf8_is_not_silently_replaced_or_dropped | 1/0/0 |
| strict_unicode_folding_maps_to_original_bytes | 1/0/0 |
| relaxed_spans_cross_separators_without_losing_offsets | 1/0/0 |
| underscore_is_a_boundary_but_a_content_suffix_is_not | 1/0/0 |
| regex_keeps_nonempty_original_byte_spans | 1/0/0 ; probe de pattern invalide séparée du panel |
| fuzzy_uses_the_existing_occurrence_definition_on_admitted_bytes | 1/0/0 |
| oracle_decode_failure_is_distinct_from_zero_matches | 1/0/0 |

Le premier juge est celui réellement vu rouge sur81b0 : builder1 au lieu de0.
La correction1ee5 est donc observée au même point consommateur avant builder,
avec le même négatif50 et les onze mêmes QueryConfig. Le positif empêche une
correction qui rejetterait toutes les vues. Les causes Read/Utf8/Query et
l’absence de témoin sont distinguées ; les spans Unicode/frontières et la
composition des cœurs sont exercés en mémoire. Le flou réemploie toujours sa
définition moteur : pas de preuve d’indépendance algorithmique ajoutée.

Les deux autres targets sont seulement compilés et inventoriés :
test_derived_in_ram1test et test_sfx_v3_ground_truth12tests dont1ignoré,
**aucun corps de test joué**, aucun builder d’index natif, corpus réel ou service.
Le loader FS historique et ses enveloppes ne sont pas requalifiés par les mocks.

## Provenance et fin effective

Les trois ELF sont rehashés physiquement en fin de pilote puis au contrôle
externe ; leurs SHA restent identiques au ledger prétest. HEAD1ee5 propre,
copies/préflight277a et marqueurcréateurT3552/5a1671 réattestés, scratch vide.

- ELF des neuf juges :**0a61a812a09a4adaead13cab8f5344965c390144bba1c6508804bff595525961**.
- Derived natif compilé :78fef16a51b354336989c4031f1032f14cd4883fab1ffbcc0762a382f7e17f17.
- Ground-truth compilé :917928a151c81a22324ca93eeb007a9aa17338781c7d2131c407f8d4fd5f14d4.

[Fin Common et contrôle externe](preuves/derived-admission-green-1ee5/final-evidence.json).
Scopevalidation-bbpomsjk, CommonVERT0/erreurnull/restes[]/nettoyétrue,
autoritéTERMINE **PID1021297/start119982634**, wrapperposte/SSH revenu0.
Common détenu absent, aucune activité owned/attribuée T/W au contrôle final ;
**12UNKNOWN nonattribués** conservés (11 en préparation), pas de quiescence
universelle revendiquée. Ressources lues et assertées par l’entry runtime :
32G/HIGHmax/swap0/cpu.max400000/100000, jobs4. L’observation externe tardive a
vu l’autorité déjà terminée ; elle ne prétend pas avoir relu un cgroup disparu.

START après poste1199826.35926876 ; boundSTART+285=1200111.35926876,
deadlineCommon pour le reste moins2 arrondi1200108.49825763. Contrôle final
externe **1199987.922879889**, avant les deux bornes ; aucun reset. Enveloppe
dure300 distincte (TERM295+kill5), aucun claim de borne285 pour tout le parcours.
La publication du pilote est contrôlée avant/aprèsopen/avantdump/aprèsflush ;
le verdict final ne repose pas sur le seul JSON provisoire du pilote.

**Restitution réelle** notifiée à C7 ET Recherche dès wrapper revenu et fin
Common/owned attestée, avant l’archivage documentaire horsposte. Recherche peut
alors prendre son propre vrai poste sous son GOFTS-ROOT-GREEN-2B5-16nov1200 ;
aucun créneau supplémentaire demandé par cette passe Lucivy. Le signal ne
prétend pas réserver ou garantir l’absence de tout autre travail sur la machine.

ELFvert copié RO sousQ/tested-green-elf et dans l’archive privée portable
`~/.cache/rag3weaver-build/cx-lucivy-admission-proof-archive-1ee5-16nov/`, hashes
vérifiés. Ancien rouge7c6f, core106fa, sources et preuves deux postes conservés.
Aucun nouveau targetCargo ni purge ; les copies ne sont pas exécutées comme
preuves. Capacité privée Common nonpubliée, autorité publique expurgée seulement.

## Portée et suite

Vert de **neuf juges d’admission mémoire**, après rouge discriminant observé.
Ce n’est pas une qualification du core entier ou d’un corpus/version. Les neuf
HOLD/ignorés du profil complet, corpus réel/FS, Linux7.2/ULP et PR42 gardent leurs
limites ; les anciens trois ciblés warnings ne sont pas rejoués dans cette passe.
Pas de main/tag/fusion/publication produit, ticket ouvert. C7 reçoit les bruts ;
Banc est sollicité pour RO des preuves seulement, sans run/ELF/proc/base/service.
