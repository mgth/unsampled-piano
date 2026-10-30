import tempfile
import tomllib
import unittest
from pathlib import Path

import numpy as np
import bench
from contact_voicing import attack, fixed_rows, interpolate, write_preset


class ContactVoicingTests(unittest.TestCase):
    def test_stereo_power_preserves_antiphase_partial_ratios(self):
        t = np.arange(12000)/bench.SR
        x = np.cos(2*np.pi*3000*t)+.1*np.cos(2*np.pi*6060*t)
        a = attack(np.column_stack([x,x]), [3000,6060,9300,13000])
        b = attack(np.column_stack([x,-x]), [3000,6060,9300,13000])
        np.testing.assert_allclose(a['partials'][:2],b['partials'][:2],atol=1e-12)
        self.assertAlmostEqual(a['partials'][0],-20,delta=.1)

    def test_true_cutoff_excludes_fourth_partial_above_12khz(self):
        t = np.arange(12000)/bench.SR
        x = np.cos(2*np.pi*2990*t)+np.cos(2*np.pi*13018*t)
        result = attack(x,[2990,6060,9400,13018])
        self.assertIsNone(result['partials'][2])

    def test_low_snr_candidate_cannot_drop_a_frozen_observation(self):
        refs = {'piano': {'108:56': {'partials':[-36,None,None]}}}
        models = {
            'base': {'108:56': {'partials':[-26,None,None], 'snr':[30,None,None]}},
            'candidate': {'108:56': {'partials':[-80,None,None], 'snr':[2,None,None]}},
        }
        rows = fixed_rows(models,refs)
        self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['values']['candidate'],-80)
        models['candidate']['108:56']['partials'][0] = None
        with self.assertRaises(ValueError): fixed_rows(models,refs)

    def test_exponent_pivot_and_log_interpolation(self):
        base = 'hardness = 0.3\nhysteresis = 0.6\np_offset = 0\ncontact_width_scale = 1\nheight_spread = 0.0001\n'
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)/'preset.toml'
            write_preset(p,base,q=.45,p=-1.25,h=.3,d=.00004)
            params = tomllib.loads(p.read_text())
            self.assertAlmostEqual(params['hardness']*.2**params['p_offset']/.3,.45,places=10)
        profile = {'96':dict(q=.25,h=.3,p=0,w=1,d=.0001),'108':dict(q=1,h=.6,p=1,w=1,d=.00004)}
        self.assertAlmostEqual(interpolate(profile,102)['q'],.5)
        self.assertAlmostEqual(interpolate(profile,102)['h'],.45)


if __name__=='__main__': unittest.main()
