"""BASELINE ROUGE NON QUALIFIÉE : point d'entrée des faux, aucun transport."""
import hashlib
from pathlib import Path

QUERY_SHA256 = hashlib.sha256(Path(__file__).with_name('identity.sql').read_bytes()).hexdigest()
POSTCHECK_SHA256 = hashlib.sha256(Path(__file__).with_name('backend-postcheck.sql').read_bytes()).hexdigest()


def validate_receipt(receipt, *, expected, ready, now, read_raw, admission=True, admitted_at=None):
    # Baseline permissive publiée pour voir les discriminants rouges avant correction.
    return receipt
