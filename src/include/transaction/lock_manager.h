#pragma once

#include <chrono>
#include <compare>
#include <condition_variable>
#include <cstdint>
#include <deque>
#include <functional>
#include <map>
#include <mutex>
#include <span>
#include <string>
#include <unordered_map>
#include <vector>

#include "common/api.h"
#include "common/types/types.h"

namespace rag3db {
namespace transaction {

// Le gestionnaire de verrous des écritures parallèles (marche V1 ; note de conception
// docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md). Comme PostgreSQL et Neo4j :
// un verrou se prend sur une ressource, en mode partagé ou exclusif, se garde jusqu'à la fin de
// la transaction, et un écrivain qui trouve la ressource prise ATTEND au lieu d'échouer. Un
// lecteur ne prend rien.
//
// Cette classe ne sait rien des tables ni du stockage : elle tient une table de ressources, la
// file d'attente de chacune et le graphe d'attente. Ce sont les marches suivantes qui
// l'appellent (A3′ pour l'insertion, A4′ pour les relations, les mises à jour et les
// suppressions, V2 pour l'annonce en tête de transaction).

enum class LockMode : uint8_t {
    // Compatible avec d'autres partagés : « personne ne détruit cette ressource sous moi ».
    SHARED = 0,
    // Compatible avec rien.
    EXCLUSIVE = 1,
};

enum class LockResourceKind : uint8_t {
    // Une ligne d'une table, désignée par sa clé (nœud) ou par son identité (relation).
    ROW = 0,
    // L'index d'une table, quand sa maintenance ne supporte qu'un écrivain à la fois.
    INDEX = 1,
    // Un genre s'ajoute ici sans rien changer d'autre : la ressource est comparée champ à
    // champ. Règle à tenir (PostgreSQL) : toute attente qui peut entrer dans un cycle doit
    // passer par ce gestionnaire, sinon la détection d'interblocage est aveugle.
};

// Une ressource : plate, générique. Deux ressources sont la même si et seulement si leurs trois
// champs sont égaux. L'ordre est total : c'est celui dans lequel une prise groupée prend ses
// verrous, quel que soit l'ordre où l'appelant les donne.
struct RAG3DB_API LockResource {
    LockResourceKind kind = LockResourceKind::ROW;
    common::table_id_t tableID = common::INVALID_TABLE_ID;
    // La clé de la ligne, sous une forme canonique que l'appelant choisit ; vide pour un index.
    std::string value;

    static LockResource row(common::table_id_t tableID, std::string key) {
        return LockResource{LockResourceKind::ROW, tableID, std::move(key)};
    }
    static LockResource index(common::table_id_t tableID) {
        return LockResource{LockResourceKind::INDEX, tableID, {}};
    }

    auto operator<=>(const LockResource&) const = default;
    std::string toString() const;
};

struct LockRequest {
    LockResource resource;
    LockMode mode = LockMode::EXCLUSIVE;
};

enum class LockOutcome : uint8_t {
    ACQUIRED = 0,
    // La demande fermerait un cycle d'attente. Détecté à la prise, tout de suite (Neo4j) :
    // c'est le demandeur qui reçoit l'erreur, et sa transaction doit être annulée.
    DEADLOCK = 1,
    // L'échéance est passée pendant l'attente.
    TIMEOUT = 2,
    // La connexion a été interrompue pendant l'attente.
    INTERRUPTED = 3,
};

class RAG3DB_API LockManager {
public:
    using clock = std::chrono::steady_clock;
    // Dit si la connexion qui attend a été interrompue ; consulté pendant l'attente.
    using interrupted_func_t = std::function<bool()>;

    // Les fragments stables des erreurs que les appelants lèvent d'après un LockOutcome, et
    // celui de l'erreur d'après attente (marche A4′). Un appelant les reconnaît à ces fragments.
    static constexpr const char* DEADLOCK_DETECTED = "deadlock detected";
    static constexpr const char* LOCK_TIMEOUT = "lock timeout";
    static constexpr const char* COULD_NOT_SERIALIZE =
        "could not serialize access due to concurrent update";

    // L'attente maximale par défaut d'un verrou : assez longue pour ne jamais gêner.
    static constexpr uint64_t DEFAULT_LOCK_TIMEOUT_IN_MS = 30'000;

    // Prend tous les verrous demandés pour la transaction, en attendant s'il le faut. Les
    // demandes sont triées par ressource (une ressource demandée deux fois l'est dans son mode
    // le plus fort) et prises dans cet ordre : deux prises groupées ne s'interbloquent pas
    // entre elles. Une transaction peut reprendre un verrou qu'elle tient, et monter un partagé
    // en exclusif.
    //
    // Sur un résultat autre qu'ACQUIRED, les verrous déjà pris restent tenus : la transaction
    // doit être annulée, et c'est releaseAll qui les rend.
    LockOutcome acquire(common::transaction_t transactionID, std::span<const LockRequest> requests,
        clock::time_point deadline, const interrupted_func_t& interrupted = nullptr);

    LockOutcome acquire(common::transaction_t transactionID, const LockResource& resource,
        LockMode mode, clock::time_point deadline, const interrupted_func_t& interrupted = nullptr) {
        const LockRequest request{resource, mode};
        return acquire(transactionID, std::span<const LockRequest>{&request, 1}, deadline,
            interrupted);
    }

    // Rend tous les verrous de la transaction et réveille ceux qui attendaient. À appeler à la
    // fin de la transaction, validée ou annulée. Sans effet si elle ne tient rien.
    void releaseAll(common::transaction_t transactionID);

    // Le message d'une erreur de verrou, avec son fragment stable.
    static std::string describe(LockOutcome outcome, const LockResource& resource);

    // Pour les tests et le diagnostic.
    uint64_t getNumLocksHeld(common::transaction_t transactionID) const;
    uint64_t getNumResources() const;
    bool isWaiting(common::transaction_t transactionID) const;

private:
    struct Waiter {
        common::transaction_t transactionID;
        LockMode mode;
    };
    struct ResourceState {
        // Qui tient la ressource, et dans quel mode. Plusieurs partagés, ou un seul exclusif.
        std::unordered_map<common::transaction_t, LockMode> holders;
        // Ceux qui attendent, dans l'ordre d'arrivée.
        std::deque<Waiter> waiters;
    };

    LockOutcome acquireOneNoLock(std::unique_lock<std::mutex>& lck,
        common::transaction_t transactionID, const LockRequest& request, clock::time_point deadline,
        const interrupted_func_t& interrupted);
    // La demande peut-elle être servie maintenant ? positionInQueue est le rang du demandeur
    // dans la file de la ressource, ou la taille de la file s'il n'y est pas encore.
    static bool canGrantNoLock(const ResourceState& state, common::transaction_t transactionID,
        LockMode mode, size_t positionInQueue);
    // Les transactions qui empêchent le demandeur d'être servi : les détenteurs incompatibles,
    // et ceux qui attendent devant lui une demande incompatible.
    static void collectBlockersNoLock(const ResourceState& state,
        common::transaction_t transactionID, LockMode mode, size_t positionInQueue,
        std::vector<common::transaction_t>& blockers);
    // En suivant le graphe d'attente depuis `from`, arrive-t-on à `target` ?
    bool reachesNoLock(common::transaction_t from, common::transaction_t target,
        std::vector<common::transaction_t>& visited) const;
    void removeWaiterNoLock(ResourceState& state, common::transaction_t transactionID);
    void eraseIfUnusedNoLock(const LockResource& resource);

    mutable std::mutex mtx;
    // Un seul signal pour toute la table : chaque libération réveille tous ceux qui attendent,
    // qui revérifient. Suffisant tant que peu de transactions attendent à la fois.
    std::condition_variable released;
    std::map<LockResource, ResourceState> resources;
    // Ce que chaque transaction tient, pour releaseAll.
    std::unordered_map<common::transaction_t, std::vector<LockResource>> heldBy;
    // La ressource que chaque transaction en attente attend (une seule à la fois).
    std::unordered_map<common::transaction_t, LockResource> waitingFor;
};

} // namespace transaction
} // namespace rag3db
