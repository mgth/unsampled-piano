"""Numerical guardrails for the diagnostic energy, independently of piano renders."""
import unittest
import numpy as np
import bench
from top_register_audit import energy_bands

class EnergyTests(unittest.TestCase):
    def test_sine_power_is_independent_of_window_length_and_channel_polarity(self):
        t=np.arange(bench.SR)/bench.SR
        x=np.sin(2*np.pi*1000*t)
        for stereo in [np.column_stack([x,x]),np.column_stack([x,-x])]:
            for duration in [.06,.1,.4]:
                self.assertAlmostEqual(energy_bands(stereo,0,0,duration)[0],10*np.log10(.5),places=5)

    def test_one_silent_channel_halves_energy_and_common_anchor_stays_fixed(self):
        t=np.arange(bench.SR)/bench.SR
        x=np.sin(2*np.pi*1000*t)
        both=np.column_stack([x,x]); one=np.column_stack([x,np.zeros_like(x)])
        self.assertAlmostEqual(energy_bands(one,0,0,.1)[0]-energy_bands(both,0,0,.1)[0],-10*np.log10(2),places=5)
        delayed=np.pad(both,((2400,0),(0,0)))
        np.testing.assert_allclose(energy_bands(delayed,2400,0,.1),energy_bands(both,0,0,.1),atol=1e-12)

    def test_truncated_windows_are_rejected(self):
        with self.assertRaises(ValueError): energy_bands(np.zeros((480,2)),0,0,.1)

if __name__=='__main__': unittest.main()
