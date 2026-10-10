#pragma once

#include <chrono>
#include <cstdint>
#include <string>

namespace rag3db {
namespace storage {

// Le verrou des lecteurs, sur un fichier posé à côté de la base (`<base>.readers`) : il fait
// céder le point de reprise d'un écrivain aux ouvertures en lecture seule d'autres processus,
// comme SQLite en mode WAL (le point de reprise s'arrête devant la marque d'un lecteur, le
// lecteur n'est jamais refusé).
//
//  - Une ouverture en lecture seule le prend PARTAGÉ le temps qu'elle lit les deux fichiers : de
//    la capture de l'en-tête et du journal jusqu'au rejeu en mémoire (WALReplayer::replay). Si un
//    point de reprise le tient, elle attend sa fin, bornée par READ_ONLY_OPEN_WAIT, et au-delà
//    son refus le dit : « waited N ms for a checkpoint of another process ».
//  - Le point de reprise le prend EXCLUSIF seulement sur la fenêtre où le fichier change pour un
//    lecteur : de la marque CHECKPOINT au journal supprimé (Checkpointer::writeCheckpoint), et à
//    la reprise d'un écrivain, le rejeu des pages fantômes et la troncature du journal
//    (WALReplayer::replay). Avant, il n'écrit que des pages que l'en-tête courant ne désigne pas
//    (pages neuves, fichier fantôme) : un lecteur ne voit rien changer. Il attend les ouvertures
//    en cours au plus CHECKPOINT_WAIT, puis passe : une ouverture qui tient le verrou au-delà
//    (un lecteur bloqué, ou une base ouverte en lecture seule DANS le processus de l'écrivain)
//    ne bloque pas l'écrivain. Le constat d'après coup (CHECKPOINT_CROSSED_READ_ONLY_OPEN) reste
//    le filet de ce cas, et de toute ouverture qui n'a pas pu prendre le verrou.
//
// Ce que le verrou ne couvre pas : un lecteur RESTÉ ouvert pendant qu'un point de reprise
// passe (il lit ses pages à la demande avec les métadonnées de son ouverture). C'est l'affaire
// de la marche 5 (une époque écrite dans le fichier), voir WALReplayer::ReadOnlyOpenIdentity.
//
// flock sous POSIX et LockFileEx sous Windows, jamais fcntl : un verrou fcntl appartient au
// processus, et ne séparerait pas une ouverture en lecture seule du point de reprise d'un
// écrivain du même processus. Un processus mort rend son verrou par le noyau.
//
// Sans fichier possible (une base en mémoire ou distante, un dossier où l'on ne peut pas créer
// le fichier), le verrou n'est pas pris et ne l'empêche pas : le filet seul joue, comme avant.
class ReadersLock {
public:
    enum class Mode : uint8_t { SHARED, EXCLUSIVE };

    static constexpr std::chrono::milliseconds READ_ONLY_OPEN_WAIT{5000};
    static constexpr std::chrono::milliseconds CHECKPOINT_WAIT{1000};

    static std::string getFilePath(const std::string& databasePath) {
        return databasePath + ".readers";
    }

    // Tente le verrou jusqu'à `bound`. Rend un verrou tenu, ou non tenu si la borne est passée
    // (timedOut()) ou si le fichier n'a pas pu être ouvert.
    static ReadersLock acquire(const std::string& databasePath, Mode mode,
        std::chrono::milliseconds bound);

    ReadersLock() = default;
    ReadersLock(const ReadersLock&) = delete;
    ReadersLock& operator=(const ReadersLock&) = delete;
    ReadersLock(ReadersLock&& other) noexcept;
    ReadersLock& operator=(ReadersLock&& other) noexcept;
    ~ReadersLock() { release(); }

    bool isHeld() const { return held; }
    bool timedOut() const { return timedOut_; }
    uint64_t waitedMs() const { return waitedMs_; }
    void release();

private:
#ifdef _WIN32
    void* handle = nullptr;
#else
    int fd = -1;
#endif
    bool held = false;
    bool timedOut_ = false;
    uint64_t waitedMs_ = 0;
};

} // namespace storage
} // namespace rag3db
