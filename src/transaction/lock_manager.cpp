#include "transaction/lock_manager.h"

#include <algorithm>

#include "common/string_format.h"

using namespace rag3db::common;

namespace rag3db {
namespace transaction {

// Le pas auquel une attente regarde l'échéance et l'interruption quand personne ne la réveille.
static constexpr std::chrono::milliseconds WAIT_SLICE{10};

std::string LockResource::toString() const {
    switch (kind) {
    case LockResourceKind::ROW:
        return stringFormat("row '{}' of table {}", value, tableID);
    case LockResourceKind::INDEX:
    default:
        return stringFormat("the index of table {}", tableID);
    }
}

std::string LockManager::describe(LockOutcome outcome, const LockResource& resource) {
    switch (outcome) {
    case LockOutcome::DEADLOCK:
        return stringFormat("Transaction aborted: {} while waiting for {}. Roll it back and run "
                            "it again.",
            DEADLOCK_DETECTED, resource.toString());
    case LockOutcome::TIMEOUT:
        return stringFormat("Transaction aborted: {} while waiting for {}. Roll it back and run "
                            "it again, or raise lock_timeout.",
            LOCK_TIMEOUT, resource.toString());
    case LockOutcome::INTERRUPTED:
        return "Interrupted.";
    case LockOutcome::ACQUIRED:
    default:
        return "";
    }
}

LockOutcome LockManager::acquire(transaction_t transactionID, std::span<const LockRequest> requests,
    clock::time_point deadline, const interrupted_func_t& interrupted) {
    // L'ordre unique des prises groupées : par ressource, chaque ressource une seule fois, dans
    // son mode le plus fort.
    std::vector<LockRequest> ordered(requests.begin(), requests.end());
    std::sort(ordered.begin(), ordered.end(), [](const LockRequest& a, const LockRequest& b) {
        if (a.resource != b.resource) {
            return a.resource < b.resource;
        }
        return a.mode > b.mode;
    });
    ordered.erase(std::unique(ordered.begin(), ordered.end(),
                      [](const LockRequest& a, const LockRequest& b) {
                          return a.resource == b.resource;
                      }),
        ordered.end());

    std::unique_lock lck{mtx};
    for (const auto& request : ordered) {
        const auto outcome = acquireOneNoLock(lck, transactionID, request, deadline, interrupted);
        if (outcome != LockOutcome::ACQUIRED) {
            return outcome;
        }
    }
    return LockOutcome::ACQUIRED;
}

LockOutcome LockManager::acquireOneNoLock(std::unique_lock<std::mutex>& lck,
    transaction_t transactionID, const LockRequest& request, clock::time_point deadline,
    const interrupted_func_t& interrupted) {
    // Les références dans une std::map restent valides quand d'autres entrées vont et viennent,
    // et une ressource n'est retirée que si personne ne la tient ni ne l'attend.
    auto& state = resources[request.resource];
    const auto mode = request.mode;

    const auto grant = [&] {
        auto [holder, isNew] = state.holders.try_emplace(transactionID, mode);
        if (isNew) {
            heldBy[transactionID].push_back(request.resource);
        } else if (mode == LockMode::EXCLUSIVE) {
            holder->second = LockMode::EXCLUSIVE;
        }
    };

    if (const auto held = state.holders.find(transactionID);
        held != state.holders.end() &&
        (held->second == LockMode::EXCLUSIVE || mode == LockMode::SHARED)) {
        return LockOutcome::ACQUIRED;
    }
    if (canGrantNoLock(state, transactionID, mode, state.waiters.size())) {
        grant();
        return LockOutcome::ACQUIRED;
    }

    // Il faut attendre. Avant de se mettre en file : la demande fermerait-elle un cycle ?
    std::vector<transaction_t> blockers;
    collectBlockersNoLock(state, transactionID, mode, state.waiters.size(), blockers);
    std::vector<transaction_t> visited;
    for (const auto blocker : blockers) {
        if (reachesNoLock(blocker, transactionID, visited)) {
            eraseIfUnusedNoLock(request.resource);
            return LockOutcome::DEADLOCK;
        }
    }

    state.waiters.push_back(Waiter{transactionID, mode});
    waitingFor.insert_or_assign(transactionID, request.resource);
    const auto leaveTheQueue = [&] {
        removeWaiterNoLock(state, transactionID);
        waitingFor.erase(transactionID);
        // La file a changé : ceux qui attendaient derrière revérifient.
        released.notify_all();
    };
    while (true) {
        const auto self = std::find_if(state.waiters.begin(), state.waiters.end(),
            [&](const Waiter& waiter) { return waiter.transactionID == transactionID; });
        const auto position = static_cast<size_t>(self - state.waiters.begin());
        if (canGrantNoLock(state, transactionID, mode, position)) {
            leaveTheQueue();
            grant();
            return LockOutcome::ACQUIRED;
        }
        if (interrupted && interrupted()) {
            leaveTheQueue();
            eraseIfUnusedNoLock(request.resource);
            return LockOutcome::INTERRUPTED;
        }
        const auto now = clock::now();
        if (now >= deadline) {
            leaveTheQueue();
            eraseIfUnusedNoLock(request.resource);
            return LockOutcome::TIMEOUT;
        }
        released.wait_until(lck, std::min(deadline, now + WAIT_SLICE));
    }
}

bool LockManager::canGrantNoLock(const ResourceState& state, transaction_t transactionID,
    LockMode mode, size_t positionInQueue) {
    const auto isHolder = state.holders.contains(transactionID);
    for (const auto& [holder, heldMode] : state.holders) {
        if (holder == transactionID) {
            continue;
        }
        if (mode == LockMode::EXCLUSIVE || heldMode == LockMode::EXCLUSIVE) {
            return false;
        }
    }
    if (isHolder) {
        // Une montée de partagé en exclusif passe devant la file : ceux qui y attendent
        // attendent ce détenteur, il ne peut pas attendre derrière eux.
        return true;
    }
    // L'ordre d'arrivée : un partagé ne double pas un exclusif qui attend (sinon les lecteurs
    // de verrou affameraient l'écrivain), un exclusif ne double personne.
    for (size_t i = 0; i < positionInQueue && i < state.waiters.size(); i++) {
        if (mode == LockMode::EXCLUSIVE || state.waiters[i].mode == LockMode::EXCLUSIVE) {
            return false;
        }
    }
    return true;
}

void LockManager::collectBlockersNoLock(const ResourceState& state, transaction_t transactionID,
    LockMode mode, size_t positionInQueue, std::vector<transaction_t>& blockers) {
    for (const auto& [holder, heldMode] : state.holders) {
        if (holder != transactionID &&
            (mode == LockMode::EXCLUSIVE || heldMode == LockMode::EXCLUSIVE)) {
            blockers.push_back(holder);
        }
    }
    if (state.holders.contains(transactionID)) {
        return; // une montée ne dépend pas de la file
    }
    for (size_t i = 0; i < positionInQueue && i < state.waiters.size(); i++) {
        const auto& waiter = state.waiters[i];
        if (waiter.transactionID != transactionID &&
            (mode == LockMode::EXCLUSIVE || waiter.mode == LockMode::EXCLUSIVE)) {
            blockers.push_back(waiter.transactionID);
        }
    }
}

bool LockManager::reachesNoLock(transaction_t from, transaction_t target,
    std::vector<transaction_t>& visited) const {
    if (from == target) {
        return true;
    }
    if (std::find(visited.begin(), visited.end(), from) != visited.end()) {
        return false;
    }
    visited.push_back(from);
    const auto waiting = waitingFor.find(from);
    if (waiting == waitingFor.end()) {
        return false; // elle n'attend rien : elle finira
    }
    const auto resource = resources.find(waiting->second);
    if (resource == resources.end()) {
        return false;
    }
    const auto& state = resource->second;
    const auto self = std::find_if(state.waiters.begin(), state.waiters.end(),
        [&](const Waiter& waiter) { return waiter.transactionID == from; });
    if (self == state.waiters.end()) {
        return false;
    }
    std::vector<transaction_t> blockers;
    collectBlockersNoLock(state, from, self->mode,
        static_cast<size_t>(self - state.waiters.begin()), blockers);
    for (const auto blocker : blockers) {
        if (reachesNoLock(blocker, target, visited)) {
            return true;
        }
    }
    return false;
}

void LockManager::removeWaiterNoLock(ResourceState& state, transaction_t transactionID) {
    std::erase_if(state.waiters,
        [&](const Waiter& waiter) { return waiter.transactionID == transactionID; });
}

void LockManager::eraseIfUnusedNoLock(const LockResource& resource) {
    const auto found = resources.find(resource);
    if (found != resources.end() && found->second.holders.empty() &&
        found->second.waiters.empty()) {
        resources.erase(found);
    }
}

void LockManager::releaseAll(transaction_t transactionID) {
    std::unique_lock lck{mtx};
    const auto held = heldBy.find(transactionID);
    if (held == heldBy.end()) {
        return;
    }
    for (const auto& resource : held->second) {
        const auto found = resources.find(resource);
        if (found == resources.end()) {
            continue;
        }
        found->second.holders.erase(transactionID);
        eraseIfUnusedNoLock(resource);
    }
    heldBy.erase(held);
    released.notify_all();
}

uint64_t LockManager::getNumLocksHeld(transaction_t transactionID) const {
    std::unique_lock lck{mtx};
    const auto held = heldBy.find(transactionID);
    return held == heldBy.end() ? 0 : held->second.size();
}

uint64_t LockManager::getNumResources() const {
    std::unique_lock lck{mtx};
    return resources.size();
}

bool LockManager::isWaiting(transaction_t transactionID) const {
    std::unique_lock lck{mtx};
    return waitingFor.contains(transactionID);
}

} // namespace transaction
} // namespace rag3db
