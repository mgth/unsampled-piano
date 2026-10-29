"""Full assembled-keyboard validation against the preceding candidate."""
import json
import numpy as np
from concurrent.futures import ThreadPoolExecutor
import keyboard_common as c
import keyboard_diagnose as kd
import adaptive_h1 as a
import adaptive_search as s
from adaptive_scan import O,FOCUS

def main():
 selection=json.loads((O/'selection.json').read_text());changed=set(map(int,selection['selected']))
 previous=json.loads((c.O/'candidate.json').read_text())
 refs=json.loads((c.O/'references.json').read_text())['notes']
 if changed:
  def job(notes):
   out=O/f'final_{notes[0]}';c.render(O/'prototype_adaptive/bundle.toml',out,notes)
   return {f'{n}|{v}':c.measure(out/f'n{n}v{v}.wav',n) for n in notes for v in c.VELOCITIES}
  with ThreadPoolExecutor(2) as pool:
   x=pool.submit(job,c.NOTES[:44]);y=pool.submit(job,c.NOTES[44:]);legacy=x.result()|y.result()
 else:legacy=previous
 adaptive={f'{n}|{v}':a.measure(legacy[f'{n}|{v}']['path'],n) for n in FOCUS for v in c.VELOCITIES}
 original=json.loads((O/'measurements.json').read_text())['rows'];targets=json.loads((O/'references.json').read_text())['references']
 both={k:dict(adaptive=row,legacy=legacy[k]) for k,row in adaptive.items()}
 for n in changed:
  assert s.guard(n,both,targets,original,previous) is None,(n,s.guard(n,both,targets,original,previous))
  assert s.score(n,both,targets,original)<selection['outcomes'][str(n)]['before']*.7
 deltas=[]
 for k,new in legacy.items():
  assert new['peak']<1,k
  if int(k.split('|')[0]) in changed:continue
  old=previous[k];x=np.array(old['levels'],float);y=np.array(new['levels'],float);mask=np.isfinite(x)&(x>=np.nanmax(x[0])-45)
  assert np.isfinite(y[mask]).all(),k
  delta=float(np.max(abs(x[mask]-y[mask])));assert delta<.3,(k,delta);deltas.append(delta)
  assert kd.assess(int(k.split('|')[0]),old,refs[k])['status']==kd.assess(int(k.split('|')[0]),new,refs[k])['status'],k
 c.save(O/'final.json',dict(legacy=legacy,adaptive=adaptive,changed=sorted(changed),max_control_delta=max(deltas),control_count=len(deltas),bundle={p.name:c.sha(p) for p in (O/'prototype_adaptive').iterdir() if p.is_file()}))
 print('validated',len(legacy),'new keys',sorted(changed),'max control delta',max(deltas),flush=True)

if __name__=='__main__':main()
