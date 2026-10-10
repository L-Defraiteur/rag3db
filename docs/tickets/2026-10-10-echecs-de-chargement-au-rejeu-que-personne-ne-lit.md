# Les échecs de chargement d'extension au rejeu, que personne ne lit

- **État** : ouvert
- **Gravité** : réponse fausse possible, et surtout un coût silencieux (index entier à rebâtir)
- **Atteignable en service** : oui — il suffit qu'un fichier d'extension noté ne soit plus là où il était
- **Touche rag3weaver** : oui, c'est lui qui paie le rebâti et lui qui pourrait le dire

## Ce que c'est

La garde 2 de la reprise (`15fcc3474`) charge les extensions notées dans
`<base>.extensions` **avant** de rejouer le journal, pour que l'index suive sa
table après une mort. Quand ce chargement échoue, `ExtensionManager` range
l'échec dans `recoveryLoadFailures` et **continue** — ce qui est le bon choix,
ça dégrade au lieu de casser. Mais `getRecoveryLoadFailures()` est public,
marqué `RAG3DB_API`, et **personne ne l'appelle** : ni ailleurs dans le moteur,
ni côté Rust. La base s'ouvre, l'index part en « détaché puis rebâti », et rien
ne dit pourquoi.

C'est le même défaut de forme que les `CatalogEvent::Warning` audités le
4 octobre : **une information existe et rien ne la consulte**.

## La recette minimale

Pas de Cypher : le défaut est dans ce qui n'est pas rendu, pas dans ce qui est
calculé. La recette est un fichier retiré.

```sh
# une base laissée par une mort pendant une écriture dans une table indexée
rm <base>.extensions        # ou : déplacer libvector.rag3db_extension
# puis rouvrir : la base s'ouvre, l'index est à rebâtir, aucun message ne le dit
```

Le seul usage interne de `recoveryLoadFailures`
(`extension_manager.cpp:120-121`) est une garde anti-doublon, pas un lecteur :
elle évite de retenter un chemin déjà échoué. Personne ne lit la liste **pour
en rendre compte**.

## Le témoin

`extension/rag3weaver/tests/e2e_arret_brutal.rs` ::
`sans_la_liste_notee_la_garde_1_reprend_la_main` (10 octobre 2026) met l'état
en scène et le mesure, par paire avec son voisin :

| cas | `<base>.extensions` | verdict |
|---|---|---|
| `une_mort_apres_insertion_vectorielle` | présent | `OuvreEtJuste` — index intact |
| `sans_la_liste_notee_la_garde_1_reprend_la_main` | retiré | `OuvreIndexRebati` — détaché puis rebâti |

Il prouve donc que la dégradation est propre. **Ce qu'il ne peut pas prouver,
et c'est l'objet du ticket** : que quelqu'un l'apprenne. Le témoin voit la
différence parce qu'il a lui-même retiré le fichier ; en service, personne ne
l'a retiré exprès et personne n'est prévenu.

## La cause

Le chemin est court et complet jusqu'au bout — sauf le dernier pas. L'échec est
attrapé, typé (`RecoveryLoadFailure { name, path, what }`), rangé, et exposé par
un accesseur public. Il ne manque que l'appelant. C'est un travail fini à
quatre-vingt-dix pour cent, ce qui est précisément la façon dont cette classe de
défaut se produit : on écrit l'instrument et on oublie le cadran.

## Ce qu'il faut pour le fermer

Deux choses, et la première suffirait :

1. **que rag3weaver le lise à l'ouverture** et le rende dans son reçu de
   montage, à côté de ce que `Catalog::ensure_vector_index` dit déjà quand il
   rebâtit un index détaché. Les deux faits vont ensemble : « l'index était en
   retard » et « voici pourquoi la reprise n'avait pas son extension ». Demande
   un passage par l'API Rust (`tools/rust_api`), qui n'expose pas l'accesseur ;
2. **que le moteur l'écrive de lui-même** sur son flux d'avertissement au
   moment du rejeu. Plus simple, mais c'est l'amont du moteur, et la règle du
   dépôt est de ne rien lui proposer — donc chez nous, dans notre copie, avec la
   même retenue que les autres gardes reprises.

Et une question à trancher avant de coder, qui n'est pas technique : **un
chargement d'extension manqué au rejeu doit-il refuser l'ouverture ?** Non pour
une extension qui ne porte aucun index de cette base — ce serait un refus pour
rien. Oui, peut-être, pour celle dont une table dépend : l'ouverture réussit
aujourd'hui en laissant un index à rebâtir, ce qui est un coût qu'on préférerait
sans doute payer sciemment. Le ticket ne tranche pas ; il demande d'abord qu'on
le **sache**.
