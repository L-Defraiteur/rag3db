package billing

type Box struct {
    Count int
    label string
}
type Policy interface {
    Retry()
}
func (b *Box) Size() int { return b.Count }
func (b *Box) Retry() {
    nested := func() int { return 1 }
    _ = nested()
}
