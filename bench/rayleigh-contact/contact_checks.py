"""Isolated contact convergence and stored one-factor acoustic ablations."""
from common import *
from concurrent.futures import ThreadPoolExecutor

def main():
 profiles=json.load(open(P/'selected.json'))['profiles'];jobs=[]
 for name,n in [('balanced',102),('contact',102),('contact_pp',102),('balanced',108),('contact',108)]:
  e=entry(n) if name=='balanced' else profiles[name][str(n)];p=preset(e,O/'mechanics'/name/f'n{n}'/'preset.toml')
  for os in [5,20]:jobs.append((name,n,os,p))
 def one(job):
  name,n,os,p=job;d=p.parent/f'os{os}';
  if '--reuse' not in sys.argv:c.run([O/'bin/contact_stages',p,d,n,'45,56,100',os])
  stats=np.genfromtxt(d/'stats.csv',delimiter=',',names=True);out={}
  for row in stats:
   v=int(row['velocity']);f=np.genfromtxt(d/f'v{v}-force.csv',delimiter=',',names=True);force=sum(f[k] for k in ['s0','s1','s2']);k=np.arange(1,4);freq=PAR[str(n)]['f0']*k*np.sqrt(1+PAR[str(n)]['B']*k*k);amp=abs(np.exp(-2j*np.pi*freq[:,None]*np.arange(len(force))/48000)@force);db=20*np.log10(amp+1e-30);modes=np.genfromtxt(d/f'v{v}-modes.csv',delimiter=',',names=True);power=np.array([sum((modes['re']**2+modes['im']**2)[modes['k']==i]) for i in [1,2,3]]);modal=10*np.log10(power+1e-30);out[v]=dict(modal_relative_db=modal-modal[0],stats={k:float(row[k]) for k in stats.dtype.names},force_db=db,force_relative_db=db-db[0])
  return f'{name}:{n}:{os}',out
 with ThreadPoolExecutor(4) as ex:out=dict(ex.map(one,jobs))
 delta={}
 for name,n in [('balanced',102),('contact',102),('contact_pp',102),('balanced',108),('contact',108)]:
  delta[f'{name}:{n}']={v:np.array(out[f'{name}:{n}:5'][v]['modal_relative_db'])-out[f'{name}:{n}:20'][v]['modal_relative_db'] for v in [45,56,100]}
 c.save(P/'contact-convergence.json',dict(captures=out,modal_relative_error_5_vs_20=delta,policy='Isolated complex modal amplitudes after contact, power summed over oscillators. No audio solver change. Sampled string-force spectrum is also retained but is NOT the convergence criterion: last-substep decimation can alias.'))
 screen=json.load(open(P/'screen-results.json'));refine=json.load(open(P/'refine-results.json'));height=json.load(open(P/'c8-height-results.json'));ablation={}
 for label,source,n,name in [('old_contact',screen,102,'control'),('felt_only',screen,102,'felt11'),('shank_only',screen,102,'mechanics01'),('both',refine,102,'r020'),('lower_hysteresis',refine,102,'r018'),('c8_10um',height,108,'height008'),('c8_15um',height,108,'height009'),('c8_20um',height,108,'height010'),('c8_25um',height,108,'height011')]:
  x=next(x for x in source if x['note']==n and x['name']==name);ablation[label]={k:x[k] for k in ['entry','mae','signed','dips']}
 c.save(P/'contact-ablation.json',ablation)
 print('Max relative modal-amplitude error below 12 kHz:',max(abs(d[k]) for name,vs in delta.items() for d in vs.values() for k in range(1,2 if name.endswith('108') else 3)))
if __name__=='__main__':main()
