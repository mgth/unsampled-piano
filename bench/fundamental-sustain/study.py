from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import sys,subprocess,json,re,hashlib
import numpy as np
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));import bench as b;import harmonic_diagnostic as hd
O=R/'out/bench/fundamental-sustain';BASE=R/'out/bench/harmonic_local/graph-comparison.toml'
VARIANTS={
 'base':{},'loss_half':{'losses.b1_offset':-5.1+np.log(.5),'losses.b2':3e-8},
 'frame_off':{'frame.coupling':0},'edge_off':{'soundboard.edge_loss':0},
 'unison2':{'unison.width_mid':2},'hit_spread05':{'unison.hit_spread':.5},
 'tilt015':{'polarization.hammer_tilt':.15},'cross01':{'polarization.cross':.1},
 'radiation03':{'polarization.radiation':.3},'sympathy_off':{'sympathetic.enabled':False},
 'room_off':{'room.enabled':False}}
def preset(fields):
 s=BASE.read_text()
 for key,v in fields.items():
  section,field=key.split('.')
  value=str(v).lower() if isinstance(v,bool) else str(float(v))
  pat=rf'(?ms)(^\[{re.escape(section)}\]\n(?:(?!^\[).)*?^{field} = )[^\n]+'
  s,n=re.subn(pat,lambda m:m[1]+value,s);assert n==1,key
 return s
notes=list(range(60,70));vel=[100]
def job(item):
 name,fields=item;d=O/name;d.mkdir(exist_ok=True);p=d/'preset.toml';p.write_text(preset(fields))
 cmd=[R/'target/release/examples/fundamental_probe',p,d,','.join(map(str,notes)),','.join(map(str,vel)),2.5,.3]
 subprocess.run(list(map(str,cmd)),check=True,capture_output=True)
 rows={}
 for n in notes:
  for v in vel:
   rows[f'{n}|{v}']={}
   for stage,suffix in [('audio',''),('force','-force')]:
    x=b.load_stereo(str(d/f'n{n}v{v}{suffix}.wav'))
    rows[f'{n}|{v}'][stage]={k:hd.pack(a) for k,a in hd.describe(x,n,fft_padding=1).items()}
 (d/'metrics.json').write_text(json.dumps(rows,indent=1));print('done',name,flush=True);return name,rows
if __name__=='__main__':
 with ThreadPoolExecutor(4) as pool:results=dict(pool.map(job,VARIANTS.items()))
 (O/'screen.json').write_text(json.dumps(dict(variants=VARIANTS,notes=notes,velocities=vel,results=results),indent=1))
 print('H1 loss from attack to 1.0-1.2s (dB)')
 for name,rows in results.items():print(name,[round(rows[f'{n}|100']['audio']['decay_self'][6][0],1) for n in notes])
 print('H1 bridge force, base:',[round(results['base'][f'{n}|100']['force']['decay_self'][6][0],1) for n in notes])
