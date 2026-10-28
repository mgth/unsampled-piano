"""Check the Plath stage comparison in the player's geometry (room/lid off).
The source coordinates are approximate and the real dummy-head response is unknown.
"""
from pathlib import Path
import sys,json
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from scipy.io import wavfile
sys.path.insert(0,str(Path(__file__).parent));import diagnose as d
R=d.R;O=d.O;P=d.P;renderer=R/'out/main-revalidation/bin/unsampled-piano-render'
inputs={'base':R/'bench/main-revalidation/effective.toml','local':R/'out/main-revalidation/presets/local/n102.toml','balanced':O/'screen/e035/preset.toml'}
def one(item):
 name,source=item;p=O/'diagnostics'/f'player-{name}.toml'
 d.c.run([renderer,'dump-preset',p,'--preset',source,'--listening',R/'presets/listening/Microphones/player.toml'])
 p.write_text(d.patch(p.read_text(),{'room.enabled':False,'soundboard.lid_angle':0,'limiter':0}))
 out=O/'diagnostics'/f'player-{name}';d.c.run([d.BIN,p,out,102,'100',5])
 sr,a=wavfile.read(out/'v100.wav');acc=np.r_[0,np.diff(a[:,3].astype(float))]*sr
 A=d.c.attack(acc,d.F);Q=d.c.attack(a[:,:2].astype(float),d.F)
 return name,dict(acceleration=A,pressure=Q,transfer_p3_re_p1=Q['partials'][1]-A['partials'][1],setup='player microphones, no room, lid off, limiter off')
with ThreadPoolExecutor(3) as ex:models=dict(ex.map(one,inputs.items()))
d.c.save(P/'player-stage-check.json',models)
for n,x in models.items():print(n,x['pressure']['partials'][:2],x['acceleration']['partials'][:2],x['transfer_p3_re_p1'],flush=True)
