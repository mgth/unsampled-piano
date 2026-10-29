"""Check horizontal dampers without changing the held candidate signal."""
import keyboard_common as c
import json
import numpy as np
from scipy.io import wavfile
selection=json.loads((c.O/'selection.json').read_text());base=json.loads((c.O/'current.json').read_text());candidate=json.loads((c.O/'candidate.json').read_text())
dampers={29:.7,30:.7,47:.7,57:.12,61:.18,77:.12}
notes=list(dampers)
fields={int(n):dict(f) for n,f in selection['selected'].items()}
for n in notes:fields[n]['damper.horizontal']=dampers[n]
path=c.bundle(c.O/'release_final_bundle',fields)
out=c.O/'release_final_render';c.render(path,out,notes)
results={}
for n in notes:
 for v in c.VELOCITIES:
  k=f'{n}|{v}';sr,w=wavfile.read(out/f'n{n}v{v}.wav');sr0,old=wavfile.read(candidate[k]['path']);sr1,b=wavfile.read(base[k]['path']);assert sr==sr0==sr1==48000
  # Small global numerical/secondary differences are measured, not hidden.
  held_delta=float(np.max(abs(w[:4*sr]-old[:4*sr])))
  assert held_delta<1e-4,(k,held_delta)
  def db(a,p,q):return float(10*np.log10(np.mean(a[int(p*sr):int(q*sr)].astype(float)**2)+1e-30))
  drop=db(w,4.3,4.5)-db(w,3.8,4);old_drop=db(b,4.3,4.5)-db(b,3.8,4)
  results[k]=dict(held_max_delta=held_delta,tail_dbfs=db(w,4.3,4.5),drop=drop,baseline_drop=old_drop,absolute_tail_delta=db(w,4.3,4.5)-db(b,4.3,4.5))
  print(k,results[k],flush=True)
c.save(c.O/'release_final.json',dict(fields=fields,results=results))

# Apply only after every measured release is within 4 dB of the previous
# release slope, or already below -90 dBFS in this fixed-gain render.
for k,row in results.items():
 assert row['drop']-row['baseline_drop']<4 or row['tail_dbfs']<-90,k
selection['selected']={str(n):f for n,f in fields.items()}
selection['release_dampers']=dampers
c.save(c.O/'selection.json',selection)
c.bundle(c.O/'prototype_keyboard',fields)
