"""Guard D4's release separately; its held signal must remain identical."""
import study as s
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import json,shutil,subprocess,math
import numpy as np
from scipy.io import wavfile
exe=s.R/'out/bench/experimental_b3b2808/build/release/examples/rayleigh_live'
source=s.O/'prototype_plateau'
def job(horizontal):
 d=s.O/f'plateau_release_{horizontal}';d.mkdir(exist_ok=True)
 for p in source.glob('*.toml'):shutil.copy2(p,d/p.name)
 if not (d/'responses.f64').exists():(d/'responses.f64').symlink_to((source/'responses.f64').resolve())
 (d/'n62.toml').write_text(s.preset({'polarization.h_coupling':.5,'polarization.cross':math.sqrt(.5)*.85,'damper.horizontal':horizontal}))
 out=d/'audio';subprocess.run(list(map(str,[exe,d/'bundle.toml',out,'62','45,56,100',4,1.5,128,1])),check=True,capture_output=True)
 result={}
 for v in [45,56,100]:
  sr,w=wavfile.read(out/f'n62v{v}.wav');_,a=wavfile.read(s.O/'plateau_candidate'/f'n62v{v}.wav');_,b=wavfile.read(s.O/'plateau_control'/f'n62v{v}.wav')
  assert np.array_equal(w[:4*sr],a[:4*sr]),'held signal changed'
  rms=lambda x:float(10*np.log10(np.mean(x[int(4.3*sr):int(4.5*sr)]**2)))
  result[v]=dict(new=rms(w),control=rms(b),delta=rms(w)-rms(b),held_identical=True)
 print(horizontal,result,flush=True);return horizontal,result
with ThreadPoolExecutor(3) as pool:results=dict(pool.map(job,[.05,.08,.12]))
(s.O/'plateau_release.json').write_text(json.dumps(results,indent=2))
