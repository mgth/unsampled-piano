"""Conservative exact-key decay alerts, not a nasality or realism score.

At least two usable real pianos; 3 dB beyond their min/max at 0.5 or 1.1 s.
For fundamentals with fewer than four periods in 60 ms, use the 0.2–0.4 s
window as anchor for decisions. Raw attack-normalised curves stay available.
"""
import keyboard_common as c
import json
import numpy as np
MARGIN=3.

def anchor(n):return 2 if c.s.b.f0(n)*.06<4 else 0

def curve(row,n):
 a=np.array(row['levels'],float)[:7,0]
 return a-a[anchor(n)]

def quality(row,n):
 a=np.array(row['levels'],float)
 i=anchor(n)
 if not np.isfinite(a[[i,3,6],0]).all():return 'H1 absent, sous le bruit ou fenêtre manquante'
 if a[i,0]<np.nanmax(a[i,:])-40:return 'H1 plus de 40 dB sous le partiel dominant'
 if max(a[3,0],a[6,0])-a[i,0]>12:return 'H1 croît de plus de 12 dB après l’ancrage'
 return None

def assess(n,model,refs):
 eligible={};excluded={}
 for nm,row in refs.items():
  if nm=='main':continue
  reason=quality(row,n)
  if reason:excluded[nm]=reason
  else:eligible[nm]=curve(row,n)
 reason=quality(model,n)
 result=dict(anchor=anchor(n),reference_count=len(eligible),excluded=excluded,model_issue=reason)
 if reason or len(eligible)<2:return result|dict(status='insufficient',directions=[],excess=[],mean_delta=[])
 target=np.array(list(eligible.values()))[:,[3,6]];value=curve(model,n)[[3,6]]
 lo=target.min(axis=0);hi=target.max(axis=0);mean=target.mean(axis=0)
 excess=np.maximum(value-hi,0)+np.minimum(value-lo,0)
 directions=['fast' if z < -MARGIN else 'slow' if z>MARGIN else 'within' for z in excess]
 status='mixed' if 'fast' in directions and 'slow' in directions else 'fast' if 'fast' in directions else 'slow' if 'slow' in directions else 'within'
 return result|dict(status=status,directions=directions,excess=excess.tolist(),mean_delta=(value-mean).tolist(),lo=lo.tolist(),hi=hi.tolist(),mean=mean.tolist(),value=value.tolist())

def main():
 refs=json.loads((c.O/'references.json').read_text())['notes'];current=json.loads((c.O/'current.json').read_text())
 rows={k:assess(int(k.split('|')[0]),r,refs[k]) for k,r in current.items()}
 notes={}
 for n in c.NOTES:
  a={v:rows[f'{n}|{v}'] for v in c.VELOCITIES}
  counts={st:sum(r['status']==st for r in a.values()) for st in ['fast','slow','mixed','within','insufficient']}
  priority=max(counts['fast'],counts['slow'])>=2
  alerts=[v for v,r in a.items() if r['status'] in ['fast','slow','mixed']]
  notes[n]=dict(name=c.s.b.name(n),counts=counts,repeated=priority,alert_velocities=alerts,max_excess=max([abs(z) for r in a.values() for z in r['excess']]+[0]))
 c.save(c.O/'diagnosis.json',dict(margin=MARGIN,rows=rows,notes=notes))
 for n,a in notes.items():
  if a['alert_velocities']:print(n,a,flush=True)
 print('coverage:',{st:sum(r['status']==st for r in rows.values()) for st in ['fast','slow','mixed','within','insufficient']})

if __name__=='__main__':main()
