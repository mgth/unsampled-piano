from search import *
import shutil
b=c.b
names=sys.argv[1:] or ['geometry','geometry_heights']
configs=json.load(open(OUT/'candidates.json'))
def one(j):
 name,n=j;d=OUT/'full'/name/'notes';d.mkdir(parents=True,exist_ok=True)
 entries=configs[name]
 if str(n) not in entries:
  for l in b.LAYERS:
   p=d/f'{b.name(n)}v{l}.wav'
   if not p.exists():p.symlink_to(ROOT/f'out/contact-voicing/full/guarded/notes/{b.name(n)}v{l}.wav')
 else:
  e=entries[str(n)];p=d.parent/f'n{n}.toml';preset(p,e['params'],e.get('extra',{}))
  c.run([c.BATCH,p,d,n,','.join(map(str,b.LAYERS.values())),20,.5])
  for l,v in b.LAYERS.items():(d/f'n{n}v{v}.wav').replace(d/f'{b.name(n)}v{l}.wav')
 print(name,n,flush=True)
with ThreadPoolExecutor(4) as pool:list(pool.map(one,[(name,n) for name in names for n in b.KEYS]))
refs=json.load(open(ROOT/'bench/references.json'));base=json.load(open(ROOT/'bench/protocol-v2/result.json'))
for name in names:
 d=OUT/'full'/name;b.survey(str(d/'notes'),str(d/'survey.csv'));metrics=b.measure_set(str(d/'notes'),str(d/'survey.csv'));z=b.score(metrics,refs)
 c.save(d/'result.json',dict(analysis_version=b.ANALYSIS_VERSION,profile=name,metrics=metrics,z=z,summary=b.summary(z)))
 b.report(name,z,{},base,str(d/'scorecard.md'));print(name,b.summary(z),flush=True)
 # C#7 is absent from standard 90-note lattice. Render a separate sustained control.
 e=configs[name].get('97');p=d/'n97.toml'
 if e:
  preset(p,e['params'],e.get('extra',{}));c.run([c.BATCH,p,d/'supplement',97,'45,56,100',20,.5])
