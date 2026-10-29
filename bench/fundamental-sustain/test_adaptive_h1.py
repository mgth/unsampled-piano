import unittest
import numpy as np
import adaptive_h1 as a

class AdaptiveFundamental(unittest.TestCase):
 def signal(self,n=21,h1=.001):
  t=np.arange(int(1.6*a.SR))/a.SR;f=a.c.s.b.f0(n)
  fundamental=h1*np.exp(-2*t)*np.sin(2*np.pi*f*t)
  harmonics=np.sin(2*np.pi*2*f*t)+.5*np.sin(2*np.pi*3*f*t)
  return np.column_stack([fundamental+harmonics,fundamental-harmonics])
 def test_weak_bass_below_strong_overtones(self):
  r=a.extract(self.signal(),21)
  self.assertTrue(all(z is not None for z in r['curve']))
  np.testing.assert_allclose(r['curve'],-40/np.log(10)*(np.array(a.TIMES)-.25),atol=.25)
 def test_absent_fundamental_is_not_invented(self):
  r=a.extract(self.signal(h1=0),21)
  self.assertTrue(any(z is None for z in r['curve']))
 def test_gain_and_channel_polarity(self):
  x=self.signal(n=26,h1=.01);r=a.extract(x,26);x[:,1]*=-1
  q=a.extract(x*.1,26)
  np.testing.assert_allclose(r['curve'],q['curve'],atol=.001)
 def test_treble_keeps_the_actual_attack_anchor(self):
  r=a.extract(self.signal(n=107,h1=.1),107)
  self.assertEqual(r['anchor'],.03)
  self.assertEqual(r['times'],[.03,.25,.5,.75,1.1])
  np.testing.assert_allclose(r['curve'],-40/np.log(10)*(np.array(r['times'])-.03),atol=.25)
 def test_noise_is_not_a_valid_tonal_decay(self):
  x=np.random.default_rng(4).normal(size=(int(1.6*a.SR),2))*.01
  r=a.extract(x,21)
  self.assertTrue(any(z is None for z in r['curve']))

if __name__=='__main__':unittest.main()
