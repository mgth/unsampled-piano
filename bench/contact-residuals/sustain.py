from search import *
import pandas as pd
b=c.b; names=['base','guarded','geometry','geometry_heights','regular'];notes=[96,97,99,102,105,108]
for name,s in [('base',c.DEFAULT),('guarded',c.interpolate(GUARD,97))]:
 p=OUT/f'sustain-{name}97.toml';preset(p,s);c.run([c.BATCH,p,OUT/'full'/name/'supplement',97,'45,56,100',20,.5])
def path(name,n,v):
 l=next(l for l,w in b.LAYERS.items() if w==v)
 if n==97:return OUT/'full'/name/'supplement'/f'n97v{v}.wav'
 d=ROOT/'out/bench/protocol-v2/notes' if name=='base' else ROOT/'out/contact-voicing/full/guarded/notes' if name=='guarded' else OUT/'full'/name/'notes'
 return d/f'{b.name(n)}v{l}.wav'
model={}
for name in names:
 model[name]={}
 for n in notes:
  for v in [45,56,100]:
   p=PARAMS[str(n)];ks=np.arange(1,5);f=p['f0']*ks*np.sqrt(1+p['B']*ks*ks)
   model[name][f'{n}:{v}']=c.attack(b.load(str(path(name,n,v)),stereo=True),f,.5,.5)
refs={};sources=json.load(open(ROOT/'bench/references.json'))['sources']
for source,src in sources.items():
 if src['role']!='target':continue
 sv=pd.read_csv(ROOT/'out/bench/refs'/f'{source}.survey.csv');refs[source]={}
 for n in notes:
  for l,v in b.LAYERS.items():
   file=b.find(str(ROOT/src['dir']),n,l);s=sv[(sv.midi==n)&(sv.layer==l)]
   if not file or s.empty:continue
   first=s.iloc[0];ks=np.arange(1,5);f=first.f0*ks*np.sqrt(1+first.B*ks*ks)
   for i,k in enumerate(ks):
    hits=s[s.k==k]
    if len(hits):f[i]=hits.iloc[0].freq
   try:r=c.attack(b.load(file,stereo=True),f,.5,.5)
   except ValueError:continue
   for i,val in enumerate(r['partials']):
    if val is None or r['snr'][i]<12:r['partials'][i]=None
   refs[source][f'{n}:{v}']=r
rows=c.fixed_rows(model,refs);scores={}
for label,pred in dict(all=lambda r:True,targets=lambda r:r['note'] in [97,99,102],ff=lambda r:r['velocity']==100,p4=lambda r:r['k']==4).items():
 sub=[r for r in rows if pred(r)];scores[label]={}
 for name in names:
  e={s:[r['values'][name]-r['target'] for r in sub if r['source']==s] for s in refs};e={s:v for s,v in e.items() if v}
  scores[label][name]=dict(mae=np.mean([np.mean(np.abs(v)) for v in e.values()]),bias=np.mean([np.mean(v) for v in e.values()]),rows=len(sub),sources=len(e),low_snr=sum(r['snr'][name]<12 for r in sub))
c.save(OUT/'sustain-check.json',dict(scores=scores,coverage_rows=rows,models=model,reference_protocol=b.ANALYSIS_VERSION));print(json.dumps(c.clean(scores),indent=1))
if __name__=='__main__':
 from scipy.io import wavfile
 for name in ['guarded','regular']:
  clips=[]
  for n in [97,99,102,108]:
   for v in [45,56,100]:
    x=b.load(str(path(name,n,v)),stereo=True)[:int(1.2*b.SR)];clips.extend([x,np.zeros((int(.3*b.SR),2))])
  wavfile.write(OUT/f'listen-{name}.wav',b.SR,np.concatenate(clips).astype('float32'))
