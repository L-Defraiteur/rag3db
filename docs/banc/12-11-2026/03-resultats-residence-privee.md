# Résidence privée : mesure réelle sur card2

Mesure du 12 novembre 2026, 23:54:31–23:54:37 Europe/Paris, sur luciepc.
Verdict commun **VERT**, durée totale 5,866447 s, code 0, aucun processus
résiduel et espace privé nettoyé. Il s’agit d’une observation unique du parcours
local, sans prétention statistique.

Suite du [protocole](02-protocole-residence-et-chauffe.md), pour le
[ticket commun de résidence](../../embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md),
qui reste ouvert : la vision dépasse ce banc. Cette preuve ne vaut pas
intégration de la branche Banc entière dans master.

## Résultat fonctionnel

Le consommateur public utilise ModelSource, acquiert un bail puis rattache le
connecteur et calcule deux plongements réels de dimension 384 avec granite-107m.
Le demandeur A est annulé pendant l’activation partagée, avant l’exécution du
fournisseur ; A reste vivant et B poursuit jusqu’au résultat. C retrouve la même
instance chaude. Après la fin de tous ces demandeurs, le fournisseur reste sain
conformément à `maintenir`. Un arrêt privé explicite permet ensuite à D de
réacquérir la même déclaration avec un nouveau processus. Deux activations
prévues, aucune relance de secours.

| Demandeur | Attente du bail | Rattachement du connecteur | Calcul de deux textes |
| --- | ---: | ---: | ---: |
| B, activation déjà engagée | 2 068,118 ms | 1,754 ms | 142,779 ms |
| C, instance chaude | 0,519 ms | 0,614 ms | 7,290 ms |
| D, nouvelle instance | 2 208,484 ms | 1,525 ms | 88,821 ms |

Les barrières volontaires du contrôleur séparent acquisition et calcul pour
qualifier le fournisseur avant son usage. L’attente B inclut la coordination ;
ces colonnes ne constituent pas une latence froide du seul connecteur.
La chauffe réussit sur 16 formes sur 16, en 1 226 ms puis 1 170 ms : lots
1/8/32/128, longueurs 40/90/190/380 mots. Le lot mesuré de deux textes ne fait
pas partie de ces formes ; toutes les formes de requête ne sont donc pas
qualifiées chaudes.

## Placement et capacité

Les deux fournisseurs, PID 1097632 (starttime 86996680) puis 1098406
(starttime 86996973), ouvrent le render node de PCI `0000:04:00.0` et présentent
des compteurs `drm-engine-gfx` positifs. Le calcul GPU est effectivement observé
sur card2 ; la déclaration Vulkan seule ne sert pas de preuve de placement.
Aucun descripteur PCI `0000:07:00.0` n’est observé dans les six captures
périodiques et les qualifications. La carte de jeu seat1 n’est pas sollicitée
par ces fournisseurs dans les observations conservées.

VRAM card2 : 17 212 403 712 octets au départ, pic à 20 181 946 368,
puis retour exact au niveau initial ; supplément maximal observé **2,766 Gio**.
Les ports et états de santé partagés sont identiques avant/après : 7878 et 7879
restent actifs, 7880 et 7882 restent absents. Aucun service partagé arrêté.
Le port privé 48329 disparaît, les groupes privés sont arrêtés et la supervision
commune ne trouve aucun résidu. Le créneau de mesure est libéré ; cela ne
préjuge pas des acquisitions ultérieures des autres sessions.

## Provenance et preuves

Runtime et binaires immuables : source
`345a402d099ae2c345e4b37a96159df970a7ee83`, sous
`~/.cache/embarquement-notes/activation/bin-345a402d0/` sur luciepc.
Fournisseur SHA256 `10f44d35479f2a6462a6db5f2dfbad2714f524eff66a27589940e4e5ad44345a` ;
consommateur `c171231c0d158fef5bd06755ca7ee22c1cb00a386d1266461ce7883ef2a6fb73`.
Identité de recette commune aux deux instances :
`362f41b4c9b57755f818d94e8fbb63d452c70d87eb55ece2287587d2405688a3`.

Racine conservée aux deux postes :
`~/.cache/rag3db-banc-notes/residence-nuit-12nov/`.

- `gpu-01-sources-v2/` : plan, contrôleur, entrée et observateur hachés ;
  plan SHA256 `a580c1711039436520dee35f3318fa605f10518189da4f3c4f731a76d401ace0`.
- `gpu-01/` : événements A–D, exécutions, captures, qualifications et arrêts.
  `fin.json` SHA256 `56b66e6a8bfb40734609bb1758c5df40793a9dbb314ae413046bf096bc40adbf`.
- `validations-gpu-01/validation-bvdguacm/` : verdict, recette et enfants de la
  primitive commune ; autorité privée conservée seulement à sa source.
- `gpu-01-services-avant.json` et `gpu-01-services-apres.json` : contrôle des
  services partagés ; `analyse-gpu01.json` : vérifications consolidées locales.

Entrée réelle : `confiner_validation.py lancer --mode mesure --delai 1200`,
avec `POSTE_MEM_MAX=32G`, sans poste extérieur. Le contrôleur hérite de
l’autorité, du scope privé et de l’échéance commune. Sources de préparation et
ancien paquet non exécuté restent conservés.

## Portée

Cette mesure qualifie l’annulation d’un demandeur vivant, le partage local,
le maintien explicite et la réacquisition après arrêt privé. Elle ne qualifie
ni crash du demandeur, ni annulation au milieu d’une inférence, ni TTL,
éviction automatique, activation distante ou bail distribué. Le modèle est
froid en processus/résidence ; le disque et les caches GPU ne sont pas déclarés
froids. Les valeurs du tableau sont des échantillons, pas des médianes ni p95.
