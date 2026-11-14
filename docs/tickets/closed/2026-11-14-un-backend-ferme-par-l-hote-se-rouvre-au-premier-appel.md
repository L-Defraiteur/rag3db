# Un backend fermé par l'hôte se rouvre au premier appel, et ce qu'on y écrit alors se perd au retour

- **État** : corrigé (14 novembre 2026) par le lot `fermeture-qui-tient` d'archi, master `1cbf1cf3a` ; fermé le 15 novembre.
- **Famille** : produit
- **Gravité** : perte. Une écriture faite dans la fenêtre d'une passe (une base fermée pour être copiée, puis remplacée) s'efface au retour, si personne ne la voit et ne la rejoue.
- **Atteignable en service** : oui. Toute passe qui ferme `memoire-longue` par `POST /backend/close` pour la copier, la jouer ailleurs et la rendre.
- **Touche rag3weaver** : oui, `src/daemon_host.rs` (`close`, `close_slot`, l'ouverture paresseuse d'un appel).
- **Pour** : archi (rag3weaver archi), qui tient l'hôte ; relevé par la session mémoire (96), demandé par l'orchestration (c7).
- **Signalé par** : la vraie passe du journal des chantiers, le 14 novembre 2026, de 18 h 19 à 18 h 43.

## Ce qu'on voyait

G a fermé le backend `memoire-longue` de l'hôte du portable à 18 h 19 min 40 s
(`POST /backend/close`, `closed: true`). La fenêtre avait été annoncée à toutes les sessions :
personne n'écrit. À 18 h 35, le juge du run d'agent `agent-1a1db20d720-9091` (un essai tty) a
écrit par l'hôte. Il a posé la Source `sources/2026-11-14-agent-1a1db20d720-9091.md`, la fiche
`bilan : essai-tty` et deux apports sur `outil : report` et `outil : step_done`.

L'hôte a rouvert la base pour ces appels, sans le dire à celui qui l'avait fermée : un .wal de
161 148 octets, un .ecrivain, et `fuser` qui nomme l'hôte. Le contrôle du retour l'a vu, et ces
écritures ont été gardées puis rejouées (page `docs/memoire/14-11-2026/02-…`). Sans ce contrôle,
elles étaient écrasées.

G l'avait déjà éprouvé le 25 octobre : tout `/call`, tout `/describe` et le `tools/list` d'un
client MCP rouvrent un backend fermé. **Une annonce n'est pas un verrou.**

## Ce qui est demandé

Un `POST /backend/close` qui porte une raison (`{"manifest", "reason": "…"}`) **refuse la
réouverture** jusqu'à un `POST /backend/open` explicite. Pendant ce temps, tout appel à ce
backend rend un refus nommé, avec la raison et l'heure de fermeture. Ce n'est pas une réouverture
silencieuse. Le crochet et le juge disent ce refus dans leur journal, et l'écriture se perd en le
disant, ou se range en file. Une fermeture sans raison garde le comportement d'aujourd'hui.

Le témoin rouge d'abord : fermer avec une raison, appeler `recall_fiches`, et voir l'appel
réussir et la base rouverte.

## Ce qui l'a fermé

`1cbf1cf3a` (le lot `fermeture-qui-tient`, son ticket :
`docs/tickets/closed/2026-11-14-une-fermeture-annoncee-ne-tient-pas-un-appel-rouvre-la-base.md`).
`POST /backend/close` prend `reason` et `until` ; avec une raison, toute ouverture est refusée — un
`/call`, un `/describe`, un hook, un juge, et l'ancienne route `/open` qu'un client MCP prend en
s'attachant — en disant « fermé : <raison>, jusqu'à <quand>, par <client> (session <id>) — rien ne le
rouvre avant un POST /backend/open explicite ». Seul `POST /backend/open` la lève ; l'heure annoncée
n'ouvre rien d'elle-même. `/etat` et `daemon_status` le disent (`held_closed`). Sans raison,
l'ancien sens demeure. Témoin :
`e2e_demon_hote::une_fermeture_avec_une_raison_tient_jusqu_a_l_ouverture_explicite` (un `put_fiche`
pendant la fermeture refusé, la base non rouverte, l'écriture refusée absente après l'ouverture).

Ce qui reste hors de ce lot : l'heure de fermeture n'est pas dans le refus (le « jusqu'à » annoncé
et la session qui a fermé, oui) ; une écriture refusée est perdue en le disant, pas rangée en file.
Pour qu'une passe en profite, elle doit fermer **avec une raison** : G, et toute passe qui copie une
base, passent `reason` à `/backend/close`.
