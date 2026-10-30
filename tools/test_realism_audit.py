"""Controls for cancellation and leakage in the diagnostic audit."""
import copy
import unittest
import numpy as np
from realism_audit import cancellation_db, score_audit, stereo_power


class AuditTest(unittest.TestCase):
    def test_downmix_is_sensitive_to_phase_but_stereo_power_is_not(self):
        t = np.arange(4800)/48000
        a = np.sin(2*np.pi*440*t)
        same = np.column_stack((a,a))
        opposite = np.column_stack((a,-a))
        self.assertAlmostEqual(cancellation_db(same), 0)
        self.assertLess(cancellation_db(opposite), -250)
        np.testing.assert_allclose(stereo_power(same), stereo_power(opposite))

    def test_channel_exchange_and_global_gain_do_not_change_loss(self):
        rng = np.random.default_rng(42)
        x = rng.normal(size=(10000,2))
        self.assertAlmostEqual(cancellation_db(x), cancellation_db(x[:,::-1]))
        self.assertAlmostEqual(cancellation_db(x), cancellation_db(x*.2))

    def test_reference_is_not_in_its_own_target_and_info_is_excluded(self):
        def metrics(x):
            return {'middle': {'attack_ff': [x,x,x,x]}}
        refs = {'sources': {
            'a': {'role':'target','metrics':metrics(0)},
            'b': {'role':'target','metrics':metrics(12)},
            'synth': {'role':'info','metrics':metrics(1000)},
        }}
        before=copy.deepcopy(refs)
        r=score_audit(metrics(6), refs)
        self.assertEqual(refs,before)
        self.assertAlmostEqual(r['model_common']['score'],0)
        # Held-out real is 12 dB away, divided by the 3 dB floor.
        self.assertAlmostEqual(r['real_leave_one_out']['a']['common']['score'],4)
        self.assertAlmostEqual(r['real_leave_one_out']['a']['model_same_targets_common']['score'],2)
        self.assertEqual(len(r['common_keys']),4)
        self.assertNotIn('synth',r['real_leave_one_out'])


if __name__ == '__main__':
    unittest.main()
