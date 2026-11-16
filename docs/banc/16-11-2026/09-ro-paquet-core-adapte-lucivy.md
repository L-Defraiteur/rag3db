# RO du paquet core adapté Lucivy : préparation, pas exécution

Ticket existant indexed-false-est-ignore-pour-un-champ-texte et ticket des
prérequis du profil core par défaut, sous docs/lucivy/tickets. Source documentaire
21b18ac2ca4dca1babe622d416430fb69ec2e18d ; APP890c86af44989f3fea6c6fa62fd40d688b4084df,
main auditéa1c0dfddfb97666aee148480f8eb3e98d00885ce. Aucun Cargo/rustc/ELF,
scanproc, décodeur de corpus, base ou service exécuté pour cette relecture.

## Pièces corroborées

13entrées du manifeste SHA256SUMS rehachées depuis les blobs git, toutes
conformes. SHA du manifeste :
8e52b6e5e4e1361a730e11cb0c2c31e890c1e4d6f0cff878506ff0bb282d3b37.
Source du pilote2fa59faafe18634a055da4e770a96d4eb87da79864332f8d2a5d8ed222a8fa14,
admission3cde2f1ebd9a038641c403e75d5a4eb425ba25fe34cf0d2b0c62361101d707a2,
préparation3c55ddb9bca00d0cc565fd4043091c071761bd02d08fbb0a6439e3f80a7fdb48 :
AST analysés sans importer/exécuter leurs modules ; hashes égaux aux copies
pinées par le preflight23dd351692d45004e23f5a92c3d9ec6d5706cb03008a6afee3bc816c6aa4b3be.

Les deux manifestes contiennent15007fichiers Linux2.6.0 et8507blobs rag3db1b22409,
soit23514. Chemins relatifs sans traversée, aucune entrée.env*/.vault/.git.
Les exclusions rag3db sont explicites : .env.example, gitlink codeparsers et lien
rust_api. Les copies sont attestées par leurs reçus de préparation/vérification,
**pas rehachées physiquement par Banc**. Les licences et leurs empreintes sont
archivées, sans invention d'un Linux7.2. La sélection Linux donne12133admissibles,
300et400chemins distincts, first300 préfixe exact de first400.

Neuf HOLD sont égaux entre le dict du pilote analysé statiquement et le préflight.
Ils ne sont pas des réussites ; compiler/inventorier un ELF ne joue pas ses tests.
Les autres ignored restent hors qualification. L'inventaire source regex283/43
est préliminaire : seuls les futurs inventaires libtest donneront les comptes
réels, y compris les cibles éventuellement absentes selon cfg.

## Admission et budget relus

Known-owner T conserve ancien créateur0c795/nonce3552, UID/dev/inode et marker
pinés. Ancien ELFvert d101 est requis dans T et dans sa copie privée ; reçu
Commonvert1575006 terminé exigé. Les fichiers helper et manifeste sont hashés,
les chemins sans liens contrôlés avant réemploi. Les relevés partiels12puis13
UNKNOWN nonattribués sont conservés, pas qualifiés absents. Owned ou activité
attribuée bloque ; ce contrat ne prouve pas la quiescence universelle.

START est pris au début de l'admission, donc après le poste extérieur ; une
seule échéanceSTART+1800. Le confineur ne reçoit que le reste moins2s, sans reset.
Le pilote exige autorité vivante, mémoire effective32G/HIGHmax, jobs bornés au
quota/affinité et4maximum, libtest2threads. Les corpus et fichiers supplémentaires
sont recontrôlés avant compilation ; le cacheV3 neuf est vérifié absent et ses
ancêtres existants doivent être privés/sans liens. Les options non relues sont
refusées, les corpus/builds non qualifiés ne sont pas récupérés en héritage.

Chaque processus conserve logs/identité ; premier code nonzero arrête avec
first-red. Une assertion arrête aussi le pilote. `budget()` relit la même
échéance ; doctests consomment le reste de cette portée, pas une deuxième1800.
Le résultat propre au pilote reste provisoire ; Common terminé sans résidu et
contrôle owned final restent requis. Le profil conserve full_core_qualified=false.
Les logs de skip éventuels et comptes doctests doivent encore être jugés après
exécution, sans déduire un core entier de code0.

## Point de lancement à expliciter avant GO

L'admission n'exécute pas chdir. Son scan attribue un cwd sous W/T : le lancer
depuis W/T peut donc s'auto-bloquer avant Cargo. Fixer le cwd de la future
commande hors W/T, idéalement Q, et le noter dans le GO canonique. Cela ne
requiert pas de changer les octets du pilote si le lanceur fixe ce cwd ; sans
cette précision, la seule commande absolue ne garantit pas cette propriété.

Les deux corrections de préparation sont documentées : alias classify manquant
dans un harnais temporaire et précontrôle exigeant un parent de cache déjà créé.
Le reçu final annonce AST et3/0du classificateur pur seulement ; Banc n'a pas
rejoué ces tests. Ils ne prouvent pas le core et ne masquent pas ces erreurs.

Avis favorable **de cohérence de préparation**, sous ce cwd explicite et les
recontrôles frais de C7 ; pas un GO. ULP/corpus7.2, PR42, fusion/tag et nouveau
run restent HOLD/hors portée. Aucun faux corpus ni golden substitué. Le ticket
reste ouvert, les9HOLD et ignorés rendent toute formule « core entier vert »
inexacte, quel que soit le futur code du superviseur.
