from run import *
d=OUT/'default-control';c.run([c.BATCH,OUT/'presets/rect_base.toml',d,','.join(map(str,b.KEYS)),'45,56,100',.2,.05]);checks=[]
for n in b.KEYS:
 for l,v in b.LAYERS.items():
  old=ROOT/f'out/bench/protocol-v2/notes/{b.name(n)}v{l}.wav';new=d/f'n{n}v{v}.wav'
  a=b.load(str(old),stereo=True)[:9600];z=b.load(str(new),stereo=True)[:9600];exact=np.array_equal(a,z)
  assert exact,(n,v,float(abs(a-z).max()));checks.append(dict(note=n,velocity=v,exact_200ms=True))
c.save(OUT/'default-control.json',dict(count=len(checks),all_exact=True,checks=checks));print(len(checks),'default bench controls bit-identical')
