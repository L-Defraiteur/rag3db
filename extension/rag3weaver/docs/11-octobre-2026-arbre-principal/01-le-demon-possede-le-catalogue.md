# Le démon possède le catalogue — la forme C, avant le code

*Arbre principal et mémoire longue, 11 octobre 2026. Pour Lucie. Rien n'est codé : cette page
attend son avis.*

**La décision** (Lucie, au réveil du 11 octobre, sur
`10-octobre-2026-arbre-principal/03-l-indexation-portee-par-le-demon.md`) : la forme C,
directement. « Le serveur MCP ne doit être qu'une interface vers un démon. » Le démon possède le
catalogue, et donc l'indexation, la commande en fond et le réacteur. Les serveurs MCP, le chat,
l'atelier et la CLI deviennent des clients minces, qui lui parlent tous de la même façon.

*Les sections marquées « à écrire » ne sont pas encore rédigées. Chacune porte le nom de la session
qui l'écrit.*

## 1. Ce que le démon tient aujourd'hui, et ce qu'il devra tenir

*Arbre principal, à écrire.*

## 2. Le démon : un processus, ses backends, ses travaux

*Arbre principal, à écrire.* Il y a un démon pour toutes les bases, et un backend se nomme par le
chemin canonique de son manifeste. Les travaux longs (index, commande en fond) se lancent, se
suivent et s'arrêtent. La reprise d'une indexation commence par la forme A : l'arrêt propre entre
deux paquets en est la première marche.

## 3. Le client mince, et une seule forme sur le fil

*Mémoire longue, à écrire.* `describe` / `call` / `events` d'un backend nommé, la même forme pour
MCP, chat, atelier et CLI ; `impl Hote for ClientDuDemon` ; ce que devient `--demon`.

## 4. Quand le démon n'est pas là

*Mémoire longue, à écrire, avec l'arbre principal pour le lancement.* Installer rag3weaver, c'est
installer un démon : soit le client le lance, soit il le dit.

## 5. Le cycle de vie du démon

*Arbre principal, à écrire.* Qui le lance, qui l'arrête, le paquet npm, un démon qui tombe et ce
qu'il emporte avec lui.

## 6. Sur le fil : transport, format, clés

*Les deux sessions, à écrire.* L'exposition par clés se fait à la frontière du démon.

## 7. Les marches, testables une à une, et leur coût

*Les deux sessions, à écrire.*

## 8. Ce qui reste à Lucie

*À écrire en dernier.*
