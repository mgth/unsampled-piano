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


if __name__=='__main__':
    unittest.main()
