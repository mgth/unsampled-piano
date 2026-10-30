from search import *
grid={};n=102;old=c.interpolate(GUARD,n)
for i,(ratio,q,p,d) in enumerate(itertools.product([.095,.105,.115],[.05,.1,.2],[0,1],[.000025,.000075])):
 s=dict(old,q=q,p=p,d=d);grid[f'geo{i:03}']=dict(note=n,params=s,extra=dict(strike_ratio=ratio))
for i,(share,freq,loss) in enumerate(itertools.product([.3,.5,.7],[150,300,600],[.1,.5])):
 grid[f'shank{i:03}']=dict(note=n,params=old,extra=dict(shank_share=share,shank_freq=freq,shank_loss=loss))
c.save(OUT/'geometry-grid.json',grid)
def one(j):
 name,e=j;return name,render(name,n,e['params'],e['extra'])
with ThreadPoolExecutor(6) as pool:
 result={}
 for name,a in pool.map(one,grid.items()):
  result[name]=a
  if len(result)%20==0:print(len(result),flush=True)
 c.save(OUT/'geometry-results.json',result)
# Doubling contact oversampling only, same instrument and listening shortcuts.
p=OUT/'102-guard.toml'
for v in [45,56,100]:
 c.run([c.PROBE,p,102,v,'contact2',OUT/f'102-{v}-contact2.wav',.2,.05])
c.save(OUT/'contact2.json',{f'102:{v}':c.measure_model(OUT/f'102-{v}-contact2.wav',PARAMS['102']) for v in [45,56,100]})
