class Base:
    pass

class Box(Base):
    typed: int = 1
    untyped = None
    class Inner:
        def leaf(self):
            return 1
    def __init__(self, label):
        self.label = label
        self.annotated: int = 2
    @property
    def size(self):
        return self.typed
    def retry(self):
        def nested():
            return 1
        return nested()
