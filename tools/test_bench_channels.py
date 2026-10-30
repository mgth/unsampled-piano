import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import bench


class ChannelPowerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        t = np.arange(4 * bench.SR) / bench.SR
        f = bench.f0(60)
        cls.x = np.exp(-t) * (np.sin(2*np.pi*f*t) + .5*np.sin(2*np.pi*2*f*t))

    def values(self, x):
        return np.r_[bench.onset(x), bench.peak_db(x), bench.onset_level(x),
                     bench.partials(x,60,0,.06,[2,3]), bench.partial_level(x,60,2),
                     bench.centroid(x,60,0,.05), bench.band_levels(x,0,[500,1000],.4),
                     bench.hf_share(x), bench.envelope(x,60,[.3,1,2,3])]

    def test_duplicate_antiphase_swap_and_silent_channel(self):
        mono = self.values(self.x)
        for y in [np.column_stack((self.x,self.x)), np.column_stack((self.x,-self.x))]:
            np.testing.assert_allclose(self.values(y),mono,atol=1e-10,equal_nan=True)
        stereo=np.column_stack((self.x,.2*np.roll(self.x,13)))
        np.testing.assert_allclose(self.values(stereo),self.values(stereo[:,::-1]),atol=1e-10,equal_nan=True)
        self.assertAlmostEqual(bench.peak_db(np.column_stack((self.x,self.x*0)))-bench.peak_db(self.x),-3.01029995664)

    def test_partial_cancelled_in_mono_is_preserved(self):
        t=np.arange(bench.SR)/bench.SR; f=bench.f0(60)
        a=np.sin(2*np.pi*f*t); b=.5*np.sin(2*np.pi*2*f*t)
        x=np.column_stack((a+b,a-b))
        ratio=bench.partials(x,60,0,.06,[2])[0]
        self.assertAlmostEqual(ratio,-6.0206,delta=.15)
        self.assertLess(bench.partials(x.mean(axis=1),60,0,.06,[2])[0],-60)

    def test_plath_fixed_gain_preserves_between_note_levels(self):
        with tempfile.TemporaryDirectory() as tmp:
            src=Path(tmp)/'src';dst=Path(tmp)/'out';src.mkdir()
            t=np.arange(100)/50000
            for key,amp in [(40,1.),(41,2.)]:
                for take,mult in enumerate([.8,1.,1.2]):
                    y=amp*mult*np.sin(2*np.pi*1000*t)
                    np.savetxt(src/f'D1_PROD08_{key}_0000_M{take:04d}.csv',
                               np.column_stack((t,t*0,t*0,y,-y)),delimiter=';',header='time;force;accel;L;R',comments='')
            subprocess.run([sys.executable,str(Path(bench.ROOT)/'tools/plath_set.py'),str(src),str(dst)],check=True,capture_output=True)
            a=bench.load(str(dst/'C4v13.wav'),stereo=True);b=bench.load(str(dst/'C#4v13.wav'),stereo=True)
            # SoX resampling rounds at about 1e-7 full scale.
            np.testing.assert_allclose(b,2*a,atol=2e-7)
            np.testing.assert_allclose(a[:,0],-a[:,1],atol=2e-7)
            m=json.loads((dst/'manifest.json').read_text())
            self.assertEqual(m['gain_per_pa'],.01)
            self.assertEqual(len(m['notes']),2)
            self.assertTrue(all(n['selected_take'].endswith('M0001.csv') for n in m['notes']))
            self.assertAlmostEqual(float(np.abs(a).max()),.01,delta=.001)

    def test_failed_plath_reimport_keeps_the_previous_set(self):
        # Peaks of 1 and 2 Pa: 0.6/Pa writes the first note, then clips on
        # the second. The set imported at 0.01/Pa must stay whole.
        with tempfile.TemporaryDirectory() as tmp:
            src=Path(tmp)/'src';dst=Path(tmp)/'out';src.mkdir()
            t=np.arange(100)/50000
            for key,amp in [(40,1.),(41,2.)]:
                y=amp*np.sin(2*np.pi*1000*t)
                np.savetxt(src/f'D1_PROD08_{key}_0000_M0000.csv',
                           np.column_stack((t,t*0,t*0,y,-y)),delimiter=';',header='time;force;accel;L;R',comments='')
            tool=[sys.executable,str(Path(bench.ROOT)/'tools/plath_set.py'),str(src),str(dst)]
            subprocess.run(tool,check=True,capture_output=True)
            before={f.name:f.read_bytes() for f in dst.iterdir()}
            r=subprocess.run(tool+['--gain-per-pa','0.6'],capture_output=True,text=True)
            self.assertNotEqual(r.returncode,0)
            self.assertIn('clipping',r.stderr)
            self.assertEqual({f.name:f.read_bytes() for f in dst.iterdir()},before)
            self.assertEqual(json.loads((dst/'manifest.json').read_text())['gain_per_pa'],.01)
            self.assertEqual(sorted(f.name for f in Path(tmp).iterdir()),['out','src'])
            # No take of that state: refused, not an empty set.
            r=subprocess.run(tool+['--state','PROD09'],capture_output=True,text=True)
            self.assertNotEqual(r.returncode,0)
            self.assertIn('no PROD09 takes',r.stderr)
            self.assertEqual({f.name:f.read_bytes() for f in dst.iterdir()},before)
            self.assertEqual(sorted(f.name for f in Path(tmp).iterdir()),['out','src'])

    def test_hf_even_finds_the_key_that_stands_out(self):
        # Every key at the same share but F4, 20 dB up in both bands.
        levels={m:[-30.0+0.1*(m%3),-40.0] for m in bench.EVEN_KEYS}
        levels[65]=[-10.0,-20.0]
        rms8,rms11,max8,max11=bench.hf_even(levels)
        self.assertAlmostEqual(max8,20.0,delta=0.2)
        self.assertAlmostEqual(max11,20.0,delta=1e-9)
        self.assertLess(rms11,5.0)
        # A band read on too few keys is not scored.
        self.assertEqual(bench.hf_even({m:levels[m] for m in range(60,66)}),[None]*4)

    def test_hf_even_share_reads_channel_power(self):
        # An 8–11 kHz burst in antiphase: lost in a mono sum, kept here.
        n=bench.SR;t=np.arange(n)/bench.SR
        y=np.where(t>0.2,np.sin(2*np.pi*440*t)+0.1*np.sin(2*np.pi*9000*t),0)*np.exp(-np.maximum(t-0.2,0)*3)
        hi=np.where(t>0.2,0.1*np.sin(2*np.pi*9000*t),0)*np.exp(-np.maximum(t-0.2,0)*3)
        x=np.column_stack((y,y-2*hi))
        share=bench.hf_even_share(x)[1]
        self.assertTrue(np.isfinite(share))
        self.assertGreater(share,-30)

    def test_hf_even_of_a_render_does_not_depend_on_its_hold(self):
        # The same attack held 1.5 s or 20 s: same reading, same coverage,
        # even for a band far below what follows it.
        t=np.arange(int(1.5*bench.SR))/bench.SR
        short=(np.sin(2*np.pi*220*t)+1e-6*np.sin(2*np.pi*9000*t)*(t<.05))*np.exp(-t)
        long=np.r_[short,np.sin(2*np.pi*220*np.arange(18*bench.SR)/bench.SR)*np.exp(-1.5)]
        a,b=bench.hf_even_share(short,gate=False),bench.hf_even_share(long,gate=False)
        self.assertTrue(np.isfinite(a).all())
        np.testing.assert_array_equal(a,b)

    def test_old_reference_protocol_is_rejected_before_render(self):
        with tempfile.TemporaryDirectory() as tmp:
            refs=Path(tmp)/'refs.json'
            refs.write_text(json.dumps({'sources':{'old':{'role':'target','metrics':{}}}}))
            with patch.object(bench,'REFS',str(refs)), patch.object(sys,'argv',['bench.py','run','--no-build']), contextlib.redirect_stderr(io.StringIO()) as err:
                with self.assertRaises(SystemExit) as ex:
                    bench.main()
                self.assertEqual(ex.exception.code,2)
                self.assertIn('recompute references',err.getvalue())

    def test_old_plath_import_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(bench,'ROOT',tmp), patch.object(bench,'REFS',str(Path(tmp)/'refs.json')), patch.object(sys,'argv',['bench.py','refs','--no-build','plath='+tmp+':nodyn']), contextlib.redirect_stderr(io.StringIO()) as err:
                with self.assertRaises(SystemExit) as ex:
                    bench.main()
                self.assertEqual(ex.exception.code,2)
                self.assertIn('fixed-gain import',err.getvalue())


class RobustnessTests(unittest.TestCase):
    def test_weak_and_missing_partials_sit_at_the_floor(self):
        t=np.arange(2*bench.SR)/bench.SR; f=bench.f0(96)
        x=np.sin(2*np.pi*f*t)+10**(-100/20)*np.sin(2*np.pi*2*f*t)+10**(-50/20)*np.sin(2*np.pi*3*f*t)
        p2,p3,p4=bench.partials(x,96,0,.06,[2,3,4])
        self.assertEqual(p2,bench.PARTIAL_FLOOR_DB)  # read at −100: counted at the floor
        self.assertEqual(p4,bench.PARTIAL_FLOOR_DB)  # absent from a clean signal: at the floor
        self.assertAlmostEqual(p3,-50,delta=1)

    def test_a_partial_lost_in_noise_above_the_floor_is_not_measured(self):
        rng=np.random.default_rng(1); t=np.arange(2*bench.SR)/bench.SR; f=bench.f0(96)
        x=np.sin(2*np.pi*f*t)+10**(-40/20)*rng.standard_normal(len(t))
        self.assertTrue(np.isnan(bench.partials(x,96,0,.06,[2])[0]))

    @staticmethod
    def note(m, lines=(), line_db=-20, dur=2.0):
        t=np.arange(int(dur*bench.SR))/bench.SR; f=bench.f0(m)
        x=sum(np.sin(2*np.pi*k*f*t)/k for k in range(1,12) if k*f<6000)*np.exp(-t)
        for g in lines: x=x+10**(line_db/20)*np.sin(2*np.pi*g*t)*np.exp(-t)
        return np.r_[np.zeros(int(.3*bench.SR)),x]

    def test_energy_between_the_partials_grows_with_a_halo(self):
        clean=bench.between_partials(self.note(60),60)
        rng=np.random.default_rng(2); x=self.note(60)
        halo=bench.between_partials(x+1e-1*rng.standard_normal(len(x))*(np.arange(len(x))>.3*bench.SR),60)
        self.assertLess(clean[0],-30)
        self.assertGreater(halo[0],clean[0]+15)

    def test_between_reads_any_silence_before_the_onset(self):
        t=np.arange(2*bench.SR)/bench.SR; f=bench.f0(60)
        x=sum(np.sin(2*np.pi*k*f*t)/k for k in range(1,12))*np.exp(-t)
        for ms in (0,190,200,205,210,220):
            with self.subTest(ms=ms):
                self.assertTrue(np.isfinite(bench.between_partials(np.r_[np.zeros(int(ms*bench.SR/1000)),x],60)).all())

    def test_between_does_not_depend_on_what_follows_its_windows(self):
        x=self.note(90,dur=6.0); s=bench.onset(x)
        whole=bench.between_partials(x,90)
        cut=bench.between_partials(x[:s+int(1.6*bench.SR)],90)
        np.testing.assert_allclose(whole,cut)

    def test_a_line_shared_by_three_keys_is_found_at_its_level(self):
        keys=bench.LINE_KEYS[:4]
        self.assertEqual(bench.fixed_lines({m:self.note(m) for m in keys}),bench.LINE_LIMIT_DB)
        # The median of the three strongest: one line alone is not enough.
        self.assertEqual(bench.fixed_lines({m:self.note(m,lines=[1530]) for m in keys}),bench.LINE_LIMIT_DB)
        level=bench.fixed_lines({m:self.note(m,lines=[1530,3170,4410]) for m in keys})
        self.assertAlmostEqual(level,-20,delta=3)
        self.assertIsNone(bench.fixed_lines({m:self.note(m) for m in keys[:2]}))

    def test_one_real_piano_is_not_a_target_except_for_dynamics(self):
        one={'sources':{'a':{'role':'target','metrics':{'top':{'env_mf':[0,0,0,-53],'dyn':[20,8]}}}}}
        z=bench.score({'top':{'env_mf':[0,0,0,-60],'dyn':[18,7]}},one)
        self.assertNotIn('top|env_mf|3s',z)
        self.assertIn('top|dyn|ff-pp',z)
        two={'sources':dict(one['sources'],b={'role':'target','metrics':{'top':{'env_mf':[0,0,0,-55]}}})}
        self.assertIn('top|env_mf|3s',bench.score({'top':{'env_mf':[0,0,0,-60]}},two))


if __name__=='__main__':
    unittest.main()
