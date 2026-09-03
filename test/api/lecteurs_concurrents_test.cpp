// Peut-on lire une base pendant qu'un autre processus y écrit ?
//
// Jusqu'ici, non : l'écrivain posait un F_WRLCK exclusif sur le fichier de
// données, et toute autre ouverture — même en lecture seule, qui ne demande
// qu'un F_RDLCK — était refusée. C'est la raison d'être du relais rag3daemon.
//
// storage_manager.cpp ne pose désormais le verrou que si l'ouverture n'est pas
// en lecture seule (report de Vela-Engineering/kuzu, 87bf0bef9). Ces tests
// mesurent ce que ça donne réellement, au lieu de le déduire du code :
//
//   1. un second processus peut-il seulement OUVRIR pendant qu'on écrit ?
//   2. ce qu'il lit est-il COHÉRENT, ou peut-il être déchiré ?
//   3. voit-il les écritures validées, ou un état figé ?
//
// Les deux derniers décident si rag3daemon peut cesser de relayer les lectures.
// Le processus fils est toujours l'écrivain : le père reste le processus de
// test, celui qui porte les assertions. Le fils n'utilise jamais ASSERT_*, il
// rend compte par mémoire partagée et sort par _exit().

#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

#include <map>

#include "api_test/api_test.h"

using namespace rag3db::testing;
using namespace rag3db::main;
using namespace rag3db::common;

namespace {

// Partagé entre le père et le fils par mmap anonyme.
struct Rapport {
    volatile int ecrivain_pret;    // le fils a ouvert en écriture et créé le schéma
    volatile int ecrivain_echoue;  // le fils n'a pas pu ouvrir
    volatile int lignes_ecrites;   // combien le fils a validé
    volatile int arret_demande;    // le père dit au fils de s'arrêter
};

Rapport* partager() {
    auto* r = static_cast<Rapport*>(mmap(nullptr, sizeof(Rapport), PROT_READ | PROT_WRITE,
        MAP_ANONYMOUS | MAP_SHARED, -1, 0));
    *r = Rapport{0, 0, 0, 0};
    return r;
}

// L'invariant : pour chaque ligne, double vaut exactement deux fois id. Une
// lecture déchirée — une ligne à demi écrite, ou deux colonnes prises dans des
// états différents — le viole. C'est ce qu'on cherche à prendre en flagrant
// délit.
constexpr const char* SCHEMA = "CREATE NODE TABLE Paire(id INT64, double INT64, PRIMARY KEY(id))";

struct Lecture {
    bool ouverture_ok = false;
    bool requete_ok = false;
    int lignes = 0;
    int incoherences = 0;
    std::string erreur; // pourquoi l'ouverture a échoué, mot pour mot
};

// Ouvre la base en lecture seule, lit tout, vérifie l'invariant. Ne lance rien.
Lecture lire(const std::string& chemin, const SystemConfig& base) {
    Lecture l;
    try {
        auto cfg = base;
        cfg.readOnly = true;
        Database db(chemin, cfg);
        Connection conn(&db);
        l.ouverture_ok = true;
        auto res = conn.query("MATCH (p:Paire) RETURN p.id, p.double");
        if (res == nullptr || !res->isSuccess()) {
            return l;
        }
        l.requete_ok = true;
        while (res->hasNext()) {
            auto t = res->getNext();
            const auto id = t->getValue(0)->getValue<int64_t>();
            const auto dbl = t->getValue(1)->getValue<int64_t>();
            l.lignes++;
            if (dbl != id * 2) {
                l.incoherences++;
            }
        }
    } catch (std::exception& e) {
        // ouverture ou lecture refusée : on garde la raison, c'est elle qui
        // dit si le refus est bénin ou s'il condamne l'usage.
        l.erreur = e.what();
    }
    return l;
}

} // namespace

class LecteursConcurrents : public ApiTest {
    // Surtout pas de base créée par le harnais : chaque test décide qui ouvre
    // quoi, et dans quel processus.
    void SetUp() override { BaseGraphTest::SetUp(); }
};

// 1. Un second processus peut-il ouvrir en lecture pendant qu'un écrivain tient
//    la base ? C'est la question que le verrou interdisait.
TEST_F(LecteursConcurrents, UnLecteurPeutOuvrirPendantQuOnEcrit) {
    if (databasePath.empty() || databasePath == ":memory:") {
        GTEST_SKIP() << "test sans objet sur une base en mémoire";
    }
    auto* rap = partager();
    const auto chemin = databasePath;
    const auto cfg_base = *systemConfig;

    pid_t fils = fork();
    ASSERT_NE(fils, -1);
    if (fils == 0) {
        try {
            auto cfg = cfg_base;
            cfg.readOnly = false;
            Database db(chemin, cfg);
            Connection conn(&db);
            conn.query(SCHEMA);
            conn.query("CREATE (:Paire {id: 1, double: 2})");
            conn.query("CHECKPOINT");
            rap->ecrivain_pret = 1;
            // On garde la base ouverte : le verrou d'écriture reste posé.
            while (!rap->arret_demande) {
                usleep(1000);
            }
        } catch (std::exception&) {
            rap->ecrivain_echoue = 1;
        }
        _exit(0);
    }

    for (int i = 0; i < 30000 && !rap->ecrivain_pret && !rap->ecrivain_echoue; i++) {
        usleep(1000);
    }
    ASSERT_EQ(rap->ecrivain_echoue, 0) << "le fils n'a pas pu ouvrir en écriture";
    ASSERT_EQ(rap->ecrivain_pret, 1) << "le fils n'a jamais signalé être prêt";

    // Le moment de vérité : l'écrivain tient la base, on ouvre en lecture.
    const auto l = lire(chemin, cfg_base);

    // Un écrivain, lui, doit toujours être refusé : on ne lève pas ce verrou-là.
    bool second_ecrivain_refuse = false;
    try {
        auto cfg = cfg_base;
        cfg.readOnly = false;
        Database db2(chemin, cfg);
    } catch (std::exception&) {
        second_ecrivain_refuse = true;
    }

    rap->arret_demande = 1;
    waitpid(fils, nullptr, 0);

    EXPECT_TRUE(l.ouverture_ok) << "un second processus ne peut toujours pas ouvrir en lecture";
    EXPECT_TRUE(l.requete_ok) << "ouverture obtenue, mais la requête a échoué";
    EXPECT_EQ(l.incoherences, 0) << "l'invariant double == id*2 est violé";
    EXPECT_TRUE(second_ecrivain_refuse)
        << "un SECOND ÉCRIVAIN a été accepté — le verrou d'écriture a sauté, ce n'est pas voulu";
}

// 2. Ce qu'il lit est-il cohérent ? Le lecteur rouvre sans cesse pendant que
//    l'écrivain insère et pose des points de reprise. On cherche une ligne
//    déchirée, un plantage, ou un refus.
TEST_F(LecteursConcurrents, CeQueLeLecteurVoitEstCoherent) {
    if (databasePath.empty() || databasePath == ":memory:") {
        GTEST_SKIP() << "test sans objet sur une base en mémoire";
    }
    constexpr int A_ECRIRE = 400;
    auto* rap = partager();
    const auto chemin = databasePath;
    const auto cfg_base = *systemConfig;

    pid_t fils = fork();
    ASSERT_NE(fils, -1);
    if (fils == 0) {
        try {
            auto cfg = cfg_base;
            cfg.readOnly = false;
            Database db(chemin, cfg);
            Connection conn(&db);
            conn.query(SCHEMA);
            rap->ecrivain_pret = 1;
            for (int i = 0; i < A_ECRIRE && !rap->arret_demande; i++) {
                auto r = conn.query("CREATE (:Paire {id: " + std::to_string(i) +
                                    ", double: " + std::to_string(i * 2) + "})");
                if (r != nullptr && r->isSuccess()) {
                    rap->lignes_ecrites = i + 1;
                }
                if (i % 25 == 0) {
                    conn.query("CHECKPOINT");
                }
            }
            conn.query("CHECKPOINT");
            while (!rap->arret_demande) {
                usleep(1000);
            }
        } catch (std::exception&) {
            rap->ecrivain_echoue = 1;
        }
        _exit(0);
    }

    for (int i = 0; i < 30000 && !rap->ecrivain_pret && !rap->ecrivain_echoue; i++) {
        usleep(1000);
    }
    ASSERT_EQ(rap->ecrivain_echoue, 0) << "le fils n'a pas pu ouvrir en écriture";
    ASSERT_EQ(rap->ecrivain_pret, 1);

    int tentatives = 0, ouvertures = 0, requetes = 0, incoherences = 0;
    int vu_max = 0, vu_min = -1, recule = 0;
    std::map<std::string, int> refus; // message d'erreur -> combien de fois
    while (tentatives < 60 && rap->lignes_ecrites < A_ECRIRE) {
        const auto l = lire(chemin, cfg_base);
        tentatives++;
        if (l.ouverture_ok) {
            ouvertures++;
        } else {
            refus[l.erreur]++;
        }
        if (l.requete_ok) {
            requetes++;
            incoherences += l.incoherences;
            if (l.lignes > vu_max) {
                vu_max = l.lignes;
            } else if (l.lignes < vu_max) {
                recule++; // le lecteur a vu MOINS qu'à un tour précédent
            }
            if (vu_min < 0 || l.lignes < vu_min) {
                vu_min = l.lignes;
            }
        }
        usleep(2000);
    }

    rap->arret_demande = 1;
    waitpid(fils, nullptr, 0);

    // Ce que la mesure a donné, en clair dans la sortie du test.
    std::cerr << "\n  --- lecteurs concurrents, mesuré ---\n"
              << "  écrites par l'écrivain : " << rap->lignes_ecrites << "\n"
              << "  tentatives d'ouverture : " << tentatives << "\n"
              << "  ouvertures réussies    : " << ouvertures << "\n"
              << "  requêtes réussies      : " << requetes << "\n"
              << "  lignes vues (min..max) : " << vu_min << ".." << vu_max << "\n"
              << "  reculs observés        : " << recule << "\n"
              << "  INCOHÉRENCES           : " << incoherences << "\n";
    for (const auto& [msg, n] : refus) {
        std::cerr << "  refus x" << n << " : " << msg << "\n";
    }
    std::cerr << "\n";

    EXPECT_GT(tentatives, 0);
    EXPECT_GT(ouvertures, 0) << "aucune ouverture en lecture n'a abouti";
    EXPECT_EQ(requetes, ouvertures) << "des lectures ont échoué après une ouverture réussie";
    EXPECT_GT(vu_max, 0) << "le lecteur n'a JAMAIS vu la moindre ligne de l'écrivain";

    // Le cœur de la mesure. Aucune ligne ne doit jamais être vue à demi écrite.
    EXPECT_EQ(incoherences, 0)
        << "LECTURE DÉCHIRÉE : une ligne a été lue avec double != id*2. Le lecteur "
           "sans verrou voit un état à demi écrit, rag3daemon ne peut pas cesser de relayer.";
    EXPECT_EQ(recule, 0) << "le lecteur a vu MOINS de lignes qu'à un tour précédent";

    // Des refus sont attendus, mais un seul est acceptable : celui par lequel le
    // moteur préfère refuser plutôt que lire un point de reprise à demi
    // installé (shadow_file.cpp:93). Tout autre refus est un vrai problème.
    for (const auto& [msg, n] : refus) {
        EXPECT_NE(msg.find("Couldn't replay shadow pages under read-only mode"), std::string::npos)
            << "refus INATTENDU, " << n << " fois : " << msg;
    }
}


// 3. Le refus est-il transitoire ? C'est lui qui décide si rag3daemon peut
//    cesser de relayer : un refus qu'une nouvelle tentative résout est une
//    gêne, un refus durable est un mur.
TEST_F(LecteursConcurrents, LeRefusSeResoutParUneNouvelleTentative) {
    if (databasePath.empty() || databasePath == ":memory:") {
        GTEST_SKIP() << "test sans objet sur une base en mémoire";
    }
    constexpr int A_ECRIRE = 400;
    auto* rap = partager();
    const auto chemin = databasePath;
    const auto cfg_base = *systemConfig;

    pid_t fils = fork();
    ASSERT_NE(fils, -1);
    if (fils == 0) {
        try {
            auto cfg = cfg_base;
            cfg.readOnly = false;
            Database db(chemin, cfg);
            Connection conn(&db);
            conn.query(SCHEMA);
            rap->ecrivain_pret = 1;
            for (int i = 0; i < A_ECRIRE && !rap->arret_demande; i++) {
                auto r = conn.query("CREATE (:Paire {id: " + std::to_string(i) +
                                    ", double: " + std::to_string(i * 2) + "})");
                if (r != nullptr && r->isSuccess()) {
                    rap->lignes_ecrites = i + 1;
                }
                // Point de reprise agressif : on maximise la fenêtre de refus.
                if (i % 5 == 0) {
                    conn.query("CHECKPOINT");
                }
            }
            while (!rap->arret_demande) {
                usleep(1000);
            }
        } catch (std::exception&) {
            rap->ecrivain_echoue = 1;
        }
        _exit(0);
    }

    for (int i = 0; i < 30000 && !rap->ecrivain_pret && !rap->ecrivain_echoue; i++) {
        usleep(1000);
    }
    ASSERT_EQ(rap->ecrivain_echoue, 0);
    ASSERT_EQ(rap->ecrivain_pret, 1);

    int cycles = 0, du_premier_coup = 0, refuses = 0, sauves_par_reprise = 0, perdus = 0;
    int reprises_totales = 0, incoherences = 0;
    while (cycles < 80 && rap->lignes_ecrites < A_ECRIRE) {
        cycles++;
        auto l = lire(chemin, cfg_base);
        if (l.ouverture_ok) {
            du_premier_coup++;
            incoherences += l.incoherences;
            continue;
        }
        refuses++;
        // Jusqu'à cinq nouvelles tentatives, avec une attente courte.
        bool sauve = false;
        for (int essai = 1; essai <= 5; essai++) {
            usleep(2000 * essai);
            reprises_totales++;
            l = lire(chemin, cfg_base);
            if (l.ouverture_ok) {
                sauve = true;
                incoherences += l.incoherences;
                break;
            }
        }
        if (sauve) {
            sauves_par_reprise++;
        } else {
            perdus++;
        }
    }

    rap->arret_demande = 1;
    waitpid(fils, nullptr, 0);

    std::cerr << "\n  --- le refus est-il transitoire ? ---\n"
              << "  cycles de lecture      : " << cycles << "\n"
              << "  réussis du premier coup: " << du_premier_coup << "\n"
              << "  refusés une fois       : " << refuses << "\n"
              << "    sauvés par reprise   : " << sauves_par_reprise << "\n"
              << "    JAMAIS obtenus       : " << perdus << "\n"
              << "  reprises consommées    : " << reprises_totales << "\n"
              << "  incohérences           : " << incoherences << "\n\n";

    EXPECT_GT(cycles, 0);
    EXPECT_EQ(incoherences, 0) << "lecture déchirée après reprise";
    EXPECT_EQ(perdus, 0)
        << "un refus n'a PAS été résolu par cinq nouvelles tentatives : il n'est pas "
           "transitoire, et rag3daemon ne peut pas se contenter de réessayer.";
}
