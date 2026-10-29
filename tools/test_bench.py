"""Run with python3 -m unittest discover -s tools -p 'test_bench.py'."""
import json
import os
import tempfile
import unittest
import io
import sys
from unittest.mock import patch

import numpy as np

import bench as b
import harmonic_diagnostic as hd


class HarmonicProfiles(unittest.TestCase):
    def test_short_attack_has_bins_for_every_low_bass_fundamental(self):
        t = np.arange(int(.2 * b.SR)) / b.SR
        for note in [21, 22, 23, 27, 28, 32]:
            with self.subTest(note=note):
                x = np.sin(2 * np.pi * b.f0(note) * t)
                self.assertTrue(np.isfinite(b.stereo_partial_levels(x, note, 0, .06, [1])).all())
                self.assertTrue(np.isnan(b.stereo_partial_levels(x, note, 0, .06, [1], fft_padding=1)).all())

    def test_partial_reference_refresh_refuses_schema_change_without_erasing_cache(self):
        with tempfile.TemporaryDirectory() as d:
            filename = os.path.join(d, 'harmonics.json')
            original = json.dumps(dict(spec={'version': 2}, sources={'a': {}, 'b': {}}))
            with open(filename, 'w') as f: f.write(original)
            with patch.object(b, 'HARMONIC_REFS', filename), patch.object(b, 'measure_profiles') as measure:
                with self.assertRaisesRegex(ValueError, 'partial refs update refused'):
                    b.refresh_profile_refs({'sources': {}}, names={'a'})
                measure.assert_not_called()
            with open(filename) as f: self.assertEqual(f.read(), original)

    def test_partial_reference_refresh_keeps_other_targets(self):
        with tempfile.TemporaryDirectory() as d:
            filename = os.path.join(d, 'harmonics.json')
            old_b = dict(dir=d, profiles={'60|4|attack': [1.] * 7})
            with open(filename, 'w') as f:
                json.dump(dict(spec=b.PROFILE_SPEC, sources={'b': old_b}), f)
            refs = {'sources': {nm: dict(role='target', dir=d) for nm in ['a','b']}}
            with patch.object(b, 'HARMONIC_REFS', filename), patch.object(b, 'measure_profiles', return_value={}):
                b.refresh_profile_refs(refs, names={'a'})
            with open(filename) as f: result = json.load(f)
            self.assertEqual(set(result['sources']), {'a','b'})
            self.assertEqual(result['sources']['b'], old_b)

    def test_failed_full_reference_refresh_keeps_previous_cache(self):
        with tempfile.TemporaryDirectory() as d:
            filename = os.path.join(d, 'harmonics.json')
            original = json.dumps(dict(spec={'version': 2}, sources={'old': {}}))
            with open(filename, 'w') as f: f.write(original)
            refs = {'sources': {nm: dict(role='target', dir=d) for nm in ['a','b']}}
            with patch.object(b, 'HARMONIC_REFS', filename), patch.object(b, 'measure_profiles', side_effect=[{}, RuntimeError('decode failed')]):
                with self.assertRaisesRegex(RuntimeError, 'decode failed'):
                    b.refresh_profile_refs(refs)
            with open(filename) as f: self.assertEqual(f.read(), original)

    def test_reuse_cannot_overwrite_baseline_with_missing_intermediate_keys(self):
        from contextlib import ExitStack, redirect_stderr
        with tempfile.TemporaryDirectory() as d, ExitStack() as stack:
            refs, baseline = os.path.join(d, 'refs.json'), os.path.join(d, 'baseline.json')
            with open(refs, 'w') as f: json.dump({'sources': {}}, f)
            original = '{"metrics": {}, "sentinel": "keep me"}'
            with open(baseline, 'w') as f: f.write(original)
            old_profiles = {f'{m}|{layer}|{phase}': [-10.] * 7
                            for m in b.KEYS for layer in b.LAYERS for phase in b.PROFILE_WINDOWS}
            for key, value in [('REFS', refs), ('BASELINE', baseline)]:
                stack.enter_context(patch.object(b, key, value))
            for key, value in [('load_profile_refs', {}), ('survey', None), ('measure_set', {}),
                               ('score', {}), ('measure_profiles', old_profiles)]:
                stack.enter_context(patch.object(b, key, return_value=value))
            stack.enter_context(patch.object(sys, 'argv', ['bench.py', 'run', '--reuse', '--save-baseline', '--out', d]))
            err = io.StringIO()
            with redirect_stderr(err), self.assertRaises(SystemExit) as raised: b.main()
            self.assertEqual(raised.exception.code, 2)
            self.assertIn('Cannot save baseline', err.getvalue())
            with open(baseline) as f: self.assertEqual(f.read(), original)

    def test_profiles_keep_delayed_onsets_and_directory_precedence(self):
        with tempfile.TemporaryDirectory() as d:
            dirs = [os.path.join(d, name) for name in ['first','second']]
            for directory in dirs:
                os.mkdir(directory)
                open(os.path.join(directory, 'F#7v13.wav'), 'w').close()
            t = np.arange(b.SR * 2) / b.SR
            x = np.sin(2*np.pi*3000*t) + .5*np.sin(2*np.pi*6000*t)
            delayed = np.pad(np.column_stack([x, x]), ((b.SR * 2, 0), (0, 0)))
            with patch.object(b, 'load_stereo', return_value=delayed) as load:
                profiles = b.measure_profiles(dirs)
            load.assert_called_once_with(os.path.join(dirs[0], 'F#7v13.wav'))
            self.assertAlmostEqual(profiles['102|13|sustain'][0], -6.02, delta=.1)

    def test_overtone_shape_is_independent_of_fundamental_and_gain(self):
        t = np.arange(int(1.6 * b.SR)) / b.SR
        upper = sum((.5 ** (k - 2)) * np.sin(2 * np.pi * 300 * k * t) for k in range(2, 9))
        base = np.sin(2 * np.pi * 300 * t)
        strong = hd.describe(2 * base + upper, 62)
        weak = hd.describe(.1 * base + upper, 62)
        scaled = hd.describe(.1 * (2 * base + upper), 62)
        np.testing.assert_allclose(strong['shape'][7, 1:], weak['shape'][7, 1:], atol=.02)
        np.testing.assert_allclose(strong['shape'], scaled['shape'], atol=.02)
        np.testing.assert_allclose(strong['decay'], scaled['decay'], atol=.02)
        self.assertGreater(strong['shape'][7, 0] - weak['shape'][7, 0], 25)

    def test_shape_anchor_requires_all_five_overtones(self):
        levels = np.arange(8, dtype=float)
        levels[3] = np.nan
        with patch.object(b, 'stereo_partial_levels', return_value=levels):
            result = hd.describe(np.zeros(100), 62)
        self.assertTrue(np.isnan(result['shape']).all())

    def test_each_partial_decay_has_its_own_fixed_attack_anchor(self):
        initial = np.array([-10., -20., np.nan, -30., -40., -50., -60., -70.])
        windows = [initial - 3 * i for i in range(len(hd.WINDOWS))]
        with patch.object(b, 'stereo_partial_levels', side_effect=windows):
            result = hd.describe(np.zeros(100), 60)
        self.assertTrue(np.isnan(result['decay_self'][:, 2]).all())
        np.testing.assert_allclose(result['decay_self'][:, 0], -3 * np.arange(len(hd.WINDOWS)))
        np.testing.assert_allclose(result['decay_self'][:, 1], result['decay_self'][:, 0])
        self.assertEqual(result['decay'][0, 1], -10.)

    def test_opposite_notes_cannot_cancel_and_velocities_stay_separate(self):
        keys = ["60|13|sustain", "63|13|sustain", "63|8|sustain"]
        reference = {k: [-10.0] * 7 for k in keys}
        model = {keys[0]: [-17.0] * 7, keys[1]: [-3.0] * 7, keys[2]: [-10.0] * 7}
        result = b.score_profiles(model, {"piano_a": reference, "piano_b": reference})
        self.assertEqual(result[keys[0]]["status"], "WATCH")
        self.assertEqual(result[keys[1]]["status"], "WATCH")
        self.assertEqual(result[keys[2]]["status"], "ok")
        self.assertEqual(b.profile_summary(result)["notes_alerted"], 2)

    def test_single_large_partial_is_off(self):
        key = "63|4|attack"
        ref = {key: [-10.0] * 7}
        result = b.score_profiles({key: [0.0] + [-10.0] * 6}, {"a": ref, "b": ref})
        self.assertEqual(result[key]["status"], "OFF")

    def test_reference_spread_and_small_changes_are_respected(self):
        key = "63|13|sustain"
        result = b.score_profiles({key: [-5.0] * 7}, {"a": {key: [-10.0] * 7}, "b": {key: [-10.0] * 7}})
        self.assertEqual(result[key]["status"], "ok")  # 5 dB < WATCH's absolute floor
        result = b.score_profiles({key: [-3.0] * 7}, {"a": {key: [-20.0] * 7}, "b": {key: [0.0] * 7}})
        self.assertEqual(result[key]["status"], "ok")

    def test_missing_noise_and_sparse_references_are_not_passes(self):
        key = "63|13|sustain"
        ref = {key: [-10.0] * 7}
        for model, targets in (({}, {"a": ref, "b": ref}),
                               ({key: [None] * 7}, {"a": ref, "b": ref}),
                               ({key: [0.0] * 7}, {"a": ref}),
                               ({key: [0.0] * 7}, {"a": {"60|13|sustain": [-10.0] * 7}})):
            with self.subTest(model=model, targets=targets):
                result = b.score_profiles(model, targets)
                self.assertEqual(result[key]["status"], "insufficient")
                json.dumps(result, allow_nan=False)

    def test_no_cross_note_interpolation(self):
        ref = {"60|13|sustain": [-10.0] * 7, "66|13|sustain": [-10.0] * 7}
        key = "63|13|sustain"
        result = b.score_profiles({key: [0.0] * 7}, {"a": ref, "b": ref})
        self.assertEqual(result[key]["status"], "insufficient")

    def test_extraction_is_gain_invariant_and_windows_are_distinct(self):
        t = np.arange(int(1.6 * b.SR)) / b.SR
        # Bin-centred in both FFTs, within F#7's tuning search window.
        f = 3000.0
        # A changing overtone, with sufficient window separation.
        amp = np.where(t < 0.2, 0.5, 0.125)
        x = np.sin(2 * np.pi * f * t) + amp * np.sin(4 * np.pi * f * t)
        for gain in (1.0, 0.01):
            attack = b.partials(x * gain, 102, 0, 0.06, [2])[0]
            sustain = b.partials(x * gain, 102, 0.5, 0.5, [2])[0]
            self.assertAlmostEqual(attack, -6.02, delta=0.3)
            self.assertAlmostEqual(sustain, -18.06, delta=0.3)

    def test_silent_recording_is_unmeasurable(self):
        with tempfile.TemporaryDirectory() as d:
            open(os.path.join(d, "D#4v13.wav"), "w").close()
            with patch.object(b, "load_stereo", return_value=np.zeros((b.SR * 2, 2))):
                profiles = b.measure_profiles([d])
            self.assertEqual(profiles["63|13|sustain"], [None] * 7)
            self.assertNotIn("64|13|sustain", profiles)

    def test_stereo_measurement_survives_opposite_channel_polarity(self):
        t = np.arange(int(1.6 * b.SR)) / b.SR
        a = np.sin(2 * np.pi * 3000 * t)
        h = 0.5 * np.sin(2 * np.pi * 6000 * t)
        same = np.column_stack([a + h, a + h])
        cancelled_h2 = np.column_stack([a + h, a - h])
        expected = b.stereo_partial_levels(same, 102, .5, .5, [1, 2])
        measured = b.stereo_partial_levels(cancelled_h2, 102, .5, .5, [1, 2])
        np.testing.assert_allclose(measured, expected, atol=.02)
        self.assertAlmostEqual(measured[1] - measured[0], -6.02, delta=.1)
        # Even total phase opposition must not lose the onset or the note.
        opposed = np.column_stack([a + h, -a - h])
        np.testing.assert_allclose(b.stereo_partial_levels(opposed, 102, .5, .5, [1, 2]), expected, atol=.02)

    def test_stereo_levels_gain_channels_and_short_recordings(self):
        t = np.arange(b.SR) / b.SR
        x = np.sin(2 * np.pi * 3000 * t)
        mono = b.stereo_partial_levels(x, 102, .1, .5, [1])
        stereo = b.stereo_partial_levels(np.column_stack([x, x]), 102, .1, .5, [1])
        np.testing.assert_allclose(mono, stereo)
        np.testing.assert_allclose(b.stereo_partial_levels(.1*x, 102, .1, .5, [1]), mono-20, atol=.01)
        self.assertTrue(np.isnan(b.stereo_partial_levels(x, 102, .8, .5, [1])).all())

    def test_info_sources_are_not_targets_and_old_schema_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            filename = os.path.join(d, "references.json")
            source = dict(dir="notes", profiles={"63|13|sustain": [-10.0] * 7})
            with open(filename, "w") as f:
                json.dump(dict(spec=b.PROFILE_SPEC, sources={"real": source, "modelled": source}), f)
            refs = {"sources": {"real": dict(role="target", dir="notes"),
                                "modelled": dict(role="info", dir="notes")}}
            with patch.object(b, "HARMONIC_REFS", filename):
                self.assertEqual(set(b.load_profile_refs(refs)), {"real"})
                with open(filename, "w") as f:
                    json.dump(dict(spec={}, sources={}), f)
                with self.assertRaisesRegex(ValueError, "Outdated"):
                    b.load_profile_refs(refs)

    def test_reference_check_excludes_the_piano_being_scored(self):
        key = "63|13|sustain"
        checks = b.profile_reference_checks({"a": {key: [-10.0] * 7}, "b": {key: [-10.0] * 7}})
        # With two sources, leaving one out cannot satisfy min_refs=2.
        self.assertEqual(checks["a"]["insufficient"], 1)
        self.assertEqual(checks["a"]["ok"], 0)

    def test_baseline_does_not_count_missing_profiles_as_cleared(self):
        key = "63|13|sustain"
        ref = {key: [-10.0] * 7}
        targets = {"a": ref, "b": ref}
        previous = b.score_profiles({key: [0.0] * 7}, targets)
        missing = b.score_profiles({}, targets)
        corrected = b.score_profiles(ref, targets)
        self.assertIn("0 cleared", "\n".join(b.profile_report(missing, previous)))
        self.assertIn("1 cleared", "\n".join(b.profile_report(corrected, previous)))


if __name__ == "__main__":
    unittest.main()
