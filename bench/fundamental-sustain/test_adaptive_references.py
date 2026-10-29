import unittest
import adaptive_references as r

def rows():return {'24|56':{'piano':{'curve':[0,-1,-2,-3]}},'25|56':{},'27|56':{'piano':{'curve':[0,-4,-5,-6]}}}
class NeighbourReferences(unittest.TestCase):
 def test_neighbours_do_not_cross_anchor_boundary_even_with_same_length(self):
  for n,lo,hi in [(39,37,40),(40,39,42)]:
   for upper in [[0,-1,-2,-3],[0,-1,-2,-3,-4]]:
    x={f'{lo}|56':{'piano':{'curve':[0,-1,-2,-3]}},f'{hi}|56':{'piano':{'curve':upper}}}
    self.assertIsNone(r.bracket(x,n,56,'piano'))
 def test_incompatible_time_grid_or_anchor_is_not_interpolated(self):
  for field,value in [('times',[.3,.5,.75,1.1]),('anchor',.03)]:
   x=rows();x['24|56']['piano'][field]=value
   self.assertIsNone(r.bracket(x,25,56,'piano'))
 def test_weighted_curve_counts_as_one_piano(self):
  x=r.make_reference(rows(),25,56,3)
  self.assertEqual(x['count'],1)
  self.assertEqual(x['estimated']['piano']['curve'],[0,-2,-3,-4])
  self.assertEqual(x['estimated']['piano']['neighbours'],[24,27])
 def test_neither_velocity_borrowing_nor_extrapolation(self):
  self.assertIsNone(r.bracket(rows(),25,100,'piano'))
  self.assertIsNone(r.bracket(rows(),23,56,'piano'))
 def test_usable_exact_data_take_precedence(self):
  x=rows();x['25|56']={'piano':{'curve':[0,-2,-3,-4]},'other':{'curve':[0,-3,-4,-5]}}
  self.assertFalse(r.make_reference(x,25,56,3)['estimated'])
 def test_large_gap_and_missing_endpoint_rejected(self):
  x=rows();x['35|56']=x.pop('27|56')
  self.assertIsNone(r.bracket(x,25,56,'piano'))
  x=rows();x['27|56']['piano']['curve'][-1]=None
  self.assertIsNone(r.bracket(x,25,56,'piano'))

if __name__=='__main__':unittest.main()
