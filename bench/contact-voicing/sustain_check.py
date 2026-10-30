import sys,json
from pathlib import Path
import numpy as np,pandas as pd
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
import bench as b
OUT=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else ROOT/'out/contact-voicing'
KEYS=[96,99,102,105,108]
PARAMS={int(k):v for k,v in json.load(open(OUT/'note-parameters.json')).items()}
from contact_voicing import attack as window
names=['base','q_only','shaped','guarded'];model={n:{} for n in names};rows=[];sources=json.load(open(ROOT/'bench/references.json'))['sources']
for name in names:
 d=ROOT/'out/bench/protocol-v2/notes' if name=='base' else OUT/'full'/name/'notes'
 for m in KEYS:
  p=PARAMS[m];ks=np.arange(1,5);f=p['f0']*ks*np.sqrt(1+p['B']*ks*ks);x=b.load(b.find(str(d),m,13),stereo=True);model[name][m]=window(x,f,.5,.5)
for source,src in sources.items():
 if src['role']!='target':continue
 sv=pd.read_csv(ROOT/'out/bench/refs'/f'{source}.survey.csv')
 for m in KEYS:
  path=b.find(str(ROOT/src['dir']),m,13);s=sv[(sv.midi==m)&(sv.layer==13)]
  if not path or s.empty:continue
  first=s.iloc[0];ks=np.arange(1,5);f=first.f0*ks*np.sqrt(1+first.B*ks*ks)
  for i,k in enumerate(ks):
   hits=s[s.k==k]
   if len(hits):f[i]=hits.iloc[0].freq
  try:r=window(b.load(path,stereo=True),f,.5,.5)
  except ValueError:continue
  for i,target in enumerate(r['partials']):
   if target is None or r['snr'][i]<12 or model['base'][m]['partials'][i] is None:continue
   rows.append(dict(source=source,note=m,k=i+2,target=target,values={n:model[n][m]['partials'][i] for n in names},snr={n:model[n][m]['snr'][i] for n in names}))
summary={}
for k in [None,2,3,4]:
 sub=[r for r in rows if k is None or r['k']==k];summary[str(k)]={}
 for n in names:
  errors={s:[r['values'][n]-r['target'] for r in sub if r['source']==s] for s in sources};errors={s:e for s,e in errors.items() if e}
  summary[str(k)][n]=dict(mae=float(np.mean([np.mean(np.abs(e)) for e in errors.values()])),bias=float(np.mean([np.mean(e) for e in errors.values()])),rows=len(sub),sources=len(errors),low_snr=int(sum(r['snr'][n]<12 for r in sub)))
(OUT/'sustain-check.json').write_text(json.dumps(dict(summary=summary,coverage_rows=rows),indent=1));print(summary)
