"""Room diagnostic on frozen upstream-4a00f00 binaries; no preset changes.
Run: python bench/room-ui-audit/audit.py [output-directory]
Seven notes, mf, 250 ms hold + 4 s release; no pedals.
Normal uses the base contact, integral uses retained experimental contacts.
Only room-on/off pairs within each mode are causal room comparisons.
This reports measurements; it is not a calibrated realism acceptance gate.
"""
from pathlib import Path
import re, subprocess, json
import numpy as np
from scipy.io import wavfile
R=Path(__file__).resolve().parents[2]
import sys
O=Path(sys.argv[1] if len(sys.argv)>1 else '/tmp/piano-reverb-probe');O.mkdir(parents=True,exist_ok=True)
B=R/'out/upstream-4a00f00/bin'
src=R/'out/upstream-4a00f00/player_actual'
dry=O/'dry';dry.mkdir(exist_ok=True)
for p in src.glob('*.toml'):
 t=p.read_text()
 if p.name=='bundle.toml':t=t.replace('impulses = "responses.f64"',f'impulses = "{src}/responses.f64"')
 else:t=t.replace('[room]\nenabled = true','[room]\nenabled = false')
 (dry/p.name).write_text(t)
impulses={}
x=np.zeros((48000*3,2),dtype=np.float32);x[0]=1
wavfile.write(O/'impulse.wav',48000,x)
for geom in ['close','player_actual']:
 subprocess.run([str(B/'rayleigh_capture'),'room',str(R/f'out/upstream-4a00f00/{geom}/base.toml'),str(O/'impulse.wav'),str(O/f'room-{geom}.wav')],check=True)
 _,y=wavfile.read(O/f'room-{geom}.wav');wet=y-x
 energy=np.mean(wet.astype(float)**2,axis=1)
 impulses[geom]={'wet_direct_db':float(10*np.log10(energy.sum()))}
 print(geom,'wet/direct dB',10*np.log10(energy.sum()),'peak',np.max(abs(wet)),flush=True)
 print('wet energy windows',[(a,b,float(10*np.log10(energy[int(a*48000):int(b*48000)].sum()+1e-100))) for a,b in [(0,.05),(.05,.1),(.1,.3),(.3,1),(1,3)]],flush=True)
for mode in ['integral','normal']:
 for room in ['dry','studio']:
  out=O/f'{mode}-{room}'
  preset=dry if room=='dry' else src
  if mode=='integral':args=[B/'rayleigh_live',preset/'bundle.toml',out,'48,60,69,81,93,102,108','56',.25,4,512,1]
  else:args=[B/'contact_batch',preset/'base.toml',out,'48,60,69,81,93,102,108','56',.25,4]
  subprocess.run(list(map(str,args)),check=True)


read=lambda p:wavfile.read(p)[1].astype(float)
pow=lambda x:np.mean(x*x)
db=lambda x:float(10*np.log10(max(x,1e-100)))
rows=[]
for mode in ['normal','integral']:
 for n in [48,60,69,81,93,102,108]:
  d=read(O/f'{mode}-dry/n{n}v56.wav');w=read(O/f'{mode}-studio/n{n}v56.wav');wet=w-d
  peak=max(pow(d[i:i+960]) for i in range(0,12000,960))
  row=dict(mode=mode,note=n,wet_direct_db=db(pow(wet)/pow(d)),level_change_db=db(pow(w)/pow(d)),dry_tail_db={},studio_tail_db={})
  for a,b in [(.3,.4),(.5,.6),(.75,1),(1.25,1.5),(2.25,2.5),(3.25,3.5)]:
   key=str(round(a-.25,2))
   row['dry_tail_db'][key]=db(pow(d[int(a*48000):int(b*48000)])/peak)
   row['studio_tail_db'][key]=db(pow(w[int(a*48000):int(b*48000)])/peak)
  rows.append(row)
  print(mode,n,'wet',round(row['wet_direct_db'],1),'gain',round(row['level_change_db'],1),'dry tail', {k:round(v,1) for k,v in row['dry_tail_db'].items()},'wet tail',{k:round(v,1) for k,v in row['studio_tail_db'].items()})
(O/'metrics.json').write_text(json.dumps(rows,indent=2)+'\n')

import hashlib
summary=dict(source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),
    bundle_sha256=hashlib.sha256((src/'bundle.toml').read_bytes()).hexdigest(),
    impulse=impulses,notes=rows,hold_s=.25,tail_s=4,velocity=56,
    tail_reference='highest 20 ms dry power window during key hold; stereo channel power',
    coverage='7 notes x 2 engines x room on/off; no pedals; diagnostic only')
(O/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
