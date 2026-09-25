import unittest
from catalog_model import ownership_model,printed_mana_value,wildcard_plan
class CatalogTests(unittest.TestCase):
 def test_printings_faces_and_rebalance(self):
  def card(i,name,n,links=[],primary=True,rebalanced=False):
   return dict(arena_id=i,name_en=name,owned=n,linked_faces=links,is_primary=primary,is_rebalanced=rebalanced,is_token=False)
  cards=[card(1,'Example',2,[2,3]),card(2,'Back',2,[1],False),card(3,'A-Example',2,[1],True,True),card(4,' EXAMPLE ',1)]
  rows=ownership_model(cards)
  self.assertEqual({r['owned_name_total'] for r in rows.values()},{3})
  self.assertEqual(rows[2]['canonical_name'],'Example')
  self.assertEqual(rows[1]['owned_printing_family'],2)
 def test_mana(self):
  for cost,want in [('{X}{B}{B}',2),('{2/W}{G/P}',3),('{3}{B}',4)]:self.assertEqual(printed_mana_value(cost),want)
 def test_missing_rebalance_link_exact_printing_only(self):
  def card(i,n,reb=False,edition='MH3',number='38'):
   return dict(arena_id=i,name_en='Ocelot Pride',owned=n,linked_faces=[],is_primary=not reb,is_rebalanced=reb,is_token=False,set=edition,collector_number=number)
  rows=ownership_model([card(1,2),card(2,2,True),card(3,1,edition='OTHER'),card(4,0,True,number='39')])
  self.assertEqual(rows[1]['printing_family_id'],rows[2]['printing_family_id'])
  self.assertEqual(rows[1]['owned_printing_family'],2)
  self.assertEqual(rows[1]['owned_name_total'],3)
  self.assertNotEqual(rows[1]['printing_family_id'],rows[3]['printing_family_id'])
  self.assertNotEqual(rows[1]['printing_family_id'],rows[4]['printing_family_id'])
 def test_budget_and_missing(self):
  inventory=dict(common=1,uncommon=4,rare=0,mythic=0,snapshot_id='s',captured_at='date')
  row=dict(name_normalized='example',canonical_name='Example',owned_name_total=3,is_basic_land=False,is_craft_candidate=True,rarity_code=2,rarity='common',owned_printing_family=0,arena_id=1)
  result=wildcard_plan([dict(name='EXAMPLE',quantity=4)],[row],inventory)
  self.assertEqual(result['wildcards_required']['common'],1)
  self.assertTrue(result['fits_snapshot_budget'])
  row['is_basic_land']=True
  self.assertEqual(wildcard_plan([dict(name='example',quantity=20)],[row],inventory)['wildcards_required']['common'],0)
  self.assertFalse(wildcard_plan([dict(name='unknown',quantity=1)],[row],inventory)['fits_snapshot_budget'])
if __name__=='__main__':unittest.main()
