class Catalog:
    @staticmethod
    def create():
        return Catalog()


def register(cls):
    return cls
