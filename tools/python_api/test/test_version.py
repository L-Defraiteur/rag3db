def test_version() -> None:
    import rag3db

    assert rag3db.version != ""
    assert rag3db.storage_version > 0
    assert rag3db.version == rag3db.__version__
