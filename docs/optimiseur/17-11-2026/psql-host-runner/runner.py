"""Baseline importable, capacité psql non implémentée ; aucun effet externe.

Ce refus explicite permet au témoin nominal d'observer l'absence de capacité
sans confondre une erreur d'import avec un échec du contrat. Pas de transport,
subprocess, lecture de secret ni publication d'attestation dans cette baseline.
"""


class RunnerUnavailable(RuntimeError):
    """La capacité d'observation psql HOST n'est pas encore implémentée."""


def observe(profile, out, *, session_factory, clock, phase_check):
    raise RunnerUnavailable('psql HOST observation capability not implemented')
