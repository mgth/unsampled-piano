import unittest
import numpy as np
from keyboard_diagnose import assess,anchor

def row(end,mid=-10):
 a=np.full((8,8),-20.);a[:,0]=[0,-2,-5,mid,-12,-14,end,end]
 return {'levels':a.tolist()}

class DecayAlerts(unittest.TestCase):
 def test_both_directions_and_real_spread(self):
  refs={'a':row(-20),'b':row(-25),'main':row(-90)}
  self.assertEqual(assess(60,row(-31),refs)['status'],'fast')
  self.assertEqual(assess(60,row(-15),refs)['status'],'slow')
  self.assertEqual(assess(60,row(-24),refs)['status'],'within')
 def test_missing_and_unstable_anchor_are_not_passes(self):
  self.assertEqual(assess(60,row(-20),{'a':row(-20)})['status'],'insufficient')
  bad=row(-20);bad['levels'][6][0]=None
  self.assertEqual(assess(60,bad,{'a':row(-20),'b':row(-20)})['status'],'insufficient')
  bad=row(20)
  self.assertEqual(assess(60,row(-20),{'a':bad,'b':row(-20)})['status'],'insufficient')
 def test_low_bass_decision_does_not_use_short_attack(self):
  self.assertEqual(anchor(21),2)
  a=row(-20);b=row(-20);a['levels'][0][0]=-80
  self.assertEqual(assess(21,a,{'a':b,'b':b})['status'],'within')
 def test_opposite_errors_do_not_cancel(self):
  self.assertEqual(assess(60,row(-10,-30),{'a':row(-20),'b':row(-20)})['status'],'mixed')

if __name__=='__main__':unittest.main()
