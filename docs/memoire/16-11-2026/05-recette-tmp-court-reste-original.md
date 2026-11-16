# Préparation seulement : TMP court, cache porté et reste original

SourceAPP734087a4e99d54331aac2f6321df9ef6da3b6d9c inchangée, pilote séparé
`cx-memoire-script-fixtures-resume-pilot-r1`. [Fils socket](../tickets/2026-11-16-le-profil-tmp-prive-depasse-la-longueur-des-sockets-des-temoins-du-pont.md),
[parent ETXTBSY](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md).
C7 autorise diagnostic privé et préparation seulement ; **aucun GO reprise**.

## Observation distincte, sans rejouer Rust

Sonde prépubliée4319b9d64f, timeout10, sousR sur btrfs/device55, nonce exclusif
0700/UID1000. Six cas :80/107/108/115octets ASCII et107/108octets UTF8. Python
AF_UNIX et appel Linux brut bind avec NUL terminal acceptent80/107, refusent108/115.
Python OSError « AF_UNIX path too long », errnoNone ; raw bind errno22/EINVAL.
Les sockets sont fermés, leurs noms retirés et le dossier de sonde absent.
Ce n'est PAS une lecture de l'errno perdu du rouge Rust, ni un défaut de produit
établi. Résultat sous journaux-cx-memoire-script-fixtures-16nov/socket-probe-4319.json.

## Admission nouvelle, aucun reset

Contrôleur `script-fixtures-resume/qualify.py --launch SHA40_NOUVEAU_PILOTE`,
seulement APRÈS RO+GO exact C7. P/Q/Sv2 ABSENTS ; source et targetv1 conservés.
PREP script ne se lance pas pour matérialiser Q avant ce GO : aucune Auth/Tv2.

INITIAL1179575.678574093 et END1186775.678574093 sont constants. Après vrai poste
32GiB/HIGHmax/swap0/quota parplace, admission floor(END-now)-3, refus<=60, reçu
borné par cet END, timeout restant+15 et final posthash/scan/prépublication sousEND.
Durée calculée depuis INITIAL original, pas7200neufs. Aucun ancien GO réutilisé.
Jobs MIN8 de l'allocation pour driver ; canonique existant conserve son comportement.

## Profil et cache

TMP neuf `R/m-<nonce>`, créationexclusive0700/UID, canonique/type/dev identiques
racine, fs nonRAM, inode/permissions relus avant runtime et au final. GuardUTF8
sur pire suffixe pont-PID10digits-compteur6digits/ecoute/pont.sock <=107octets.
Ce TMP et ses traces restent conservés, sans effacement au fil ; scan final
exe/cwd/FD inclut aussi ce TMP, ownedUNKNOWN bloque. Les états XDG et RAG demeurent
frères sousP ; pas de gabarit, SDK, source ni produit modifié.

Avant attente poste et juste avant premier runtime : capture source734propre,
Tdev/inode/modeUID/markerexact, premier rouge/profil/config originaux vs26manifest
904832...5029, reçuTERMINE/ENDexact et verdictROUGE1 calme, ELF réelSHA6f1227...8e74
vs copie postrouge, fingerprints contenusSHA et vraie sonde **RO** build_rs_d_un_autre_arbre.
Snapshot publié AVANT assertions ; comparaison baseline après poste. Aucun purge,
import FP ni adoption. Le canonique appellera la vraie garde same-owner comme
avant : ce n'est ni un target neuf ni un changement de propriétaire. Les empreintes
peuvent évoluer ensuite à cause de la compilation légitime, pas avant la reprise.

Canonique entier SUITES=-/codex_threads puis F8/preuve/lib/NAV comme v1, suivi du
même ELF large entier direct600MINEND,0deuxièmeCargo/F8/NAV. Anciens rouges restent
rouges ; nouveaux logs et résumé canonique séparés, aucun assemblage historique.
Pin59ac/storeexplicite/SOhash/profilCPU/services canoniques inchangés ; aucune
commande7970 ni restart/service/base de Lucie. Stop premier rouge, pasretry.

Primitives watcher ED13c8 first_red/active_logs/watch_logs restent BYTEIDENTIQUES,
provenance recopiée. Témoins prépubliés avant exécution : 11qualification conservés,
11nouveaux profils/cache purs/fauxmetadata ; aucun Rust/Cargo/proc/signal réel.
Le contrôleur actuel doit recevoir RO et GO avant toute création de P/Q/Sv2.
