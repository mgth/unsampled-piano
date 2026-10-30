from search import *
result={}
a=json.load(open(OUT/'ablation-results.json'))
for n in [97,99,102]:
 for v in [45,56,100]:
  k=f'{n}:{v}'
  for label in ['base','guard']:
   src='base' if label=='base' else 'guarded'
   forces=np.loadtxt(OUT/f'{n}-{v}-{label}-forces.csv',delimiter=',',skiprows=1)[:,1:]
   par=PARAMS[str(n)];ks=np.arange(1,4);fr=par['f0']*ks*np.sqrt(1+par['B']*ks*ks)
   ft=np.exp(-2j*np.pi*fr[:,None]*np.arange(len(forces))[None,:]/48000)@forces
   coherent=abs(ft.sum(axis=1))**2;incoherent=np.sum(abs(ft)**2,axis=1)
   bridge=c.measure_model(OUT/f'{n}-{v}-{label}-bridge.wav',par)
   result[f'{k}:{label}']=dict(force_coherent_db=10*np.log10(coherent[1:]/coherent[0]),force_power_sum_db=10*np.log10(incoherent[1:]/incoherent[0]),bridge=bridge,radiated=MODELS[src][k],contact=(OUT/f'{n}-{v}-{label}-contact.txt').read_text().strip())
c.save(OUT/'diagnosis.json',result)
# Additional targeted one-factor variants preserving each key's q/p/h.
grid={}
for n in [97,99]:
 for d in [.000025,.00004,.00005,.00006,.000075]:
  s=c.interpolate(GUARD,n);s['d']=d;grid[f'n{n}-height-{d}']=dict(note=n,params=s)
c.save(OUT/'height-grid.json',grid)
def one(j):
 name,e=j;return name,render(name,e['note'],e['params'])
with ThreadPoolExecutor(4) as pool:c.save(OUT/'height-results.json',dict(pool.map(one,grid.items())))
