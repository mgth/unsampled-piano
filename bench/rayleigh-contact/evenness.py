"""Additional level guard: key level relative to fixed neighboring keys.
No per-note gain fit. Same source/key/velocity identities for every model.
"""
from common import *
import importlib.util
_spec=importlib.util.spec_from_file_location("contact_validation",P/"validate.py")
_validation=importlib.util.module_from_spec(_spec);_spec.loader.exec_module(_validation)
file,NAMES=_validation.file,_validation.NAMES

def main():
 refs=json.load(open(R/'bench/main-revalidation/references.json'));sources=json.load(open(R/'bench/references.json'))['sources'];levels={};observations=[]
 for source,entries in refs.items():
  levels[source]={}
  for key in entries:
   n,v=map(int,key.split(':'));layer=next(k for k,val in c.b.LAYERS.items() if val==v);path=c.b.find(str(R/sources[source]['dir']),n,layer)
   if not path:continue
   x=c.b.load(path,stereo=True);levels[source][key]=float(c.b.onset_level(np.pad(x,((1024,0),(0,0)))))
 c.save(P/'reference-levels.json',levels)
 model={name:{} for name in NAMES}
 for name in NAMES:
  for n in range(96,109):
   for v in [45,56,100]:
    _,x=wavfile.read(file('attack',name,n,v));model[name][f'{n}:{v}']=float(c.b.onset_level(np.pad(x,((1024,0),(0,0)))))
 for source,entries in levels.items():
  for n,neighbors in [(102,[99,100,101,103,104,105]),(108,[105,106,107])]:
   for v in [45,56,100]:
    key=f'{n}:{v}';anchor=[f'{m}:{v}' for m in neighbors if f'{m}:{v}' in entries]
    if key not in entries or len(anchor)<2:continue
    target=entries[key]-np.median([entries[k] for k in anchor]);values={name:model[name][key]-np.median([model[name][k] for k in anchor]) for name in NAMES};observations.append(dict(source=source,note=n,velocity=v,anchor=anchor,target=target,values=values))
 summary={}
 for n in [102,108]:
  summary[n]={}
  for v in [45,56,100]:
   rows=[r for r in observations if r['note']==n and r['velocity']==v]
   summary[n][v]={name:dict(bias=float(np.mean([r['values'][name]-r['target'] for r in rows])),mae=float(np.mean([abs(r['values'][name]-r['target']) for r in rows])),rows=len(rows)) for name in NAMES}
 c.save(P/'level-evenness.json',dict(model_levels=model,rows=observations,summary=summary,policy='Within-source contrasts against the same available neighbor set; same source eligibility across candidates. Exploratory additional guard, not a replacement for the frozen partial score.'))
 print(json.dumps(summary,indent=1))
if __name__=='__main__':main()
