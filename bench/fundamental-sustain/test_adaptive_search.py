import unittest
import adaptive_search as s

class InitialH1Guard(unittest.TestCase):
 def test_unreadable_previous_anchor_is_rejected_without_arithmetic(self):
  for value in [None,float('nan'),float('inf')]:
   key='24|45';new={'curve':[0,-2,-4,-8],'windows':[{'level':-25.}]}
   previous={'curve':[None]*4,'windows':[{'level':value}]}
   self.assertEqual(s.guard(24,{key:{'adaptive':new}},{},{key:{'previous':previous}},{}),
                    'niveau initial H1 témoin illisible')
 def test_unreadable_candidate_anchor_is_rejected(self):
  key='24|45';new={'curve':[0,-2,-4,-8],'windows':[{'level':None}]}
  previous={'windows':[{'level':-25.}]}
  self.assertEqual(s.guard(24,{key:{'adaptive':new}},{},{key:{'previous':previous}},{}),
                   'niveau initial H1 candidat illisible')

if __name__=='__main__':unittest.main()
