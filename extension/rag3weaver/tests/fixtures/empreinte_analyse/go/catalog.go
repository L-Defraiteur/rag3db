package catalog

type Catalog struct{ taille int }

func New() *Catalog { return &Catalog{taille: 1} }

func (c *Catalog) Grandir() {
	total := c.taille + 1
	c.taille = total
}
