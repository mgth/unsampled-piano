from pathlib import Path
import sys,json,re,shutil
from concurrent.futures import ThreadPoolExecutor
import numpy as np,pandas as pd
from scipy.io import wavfile
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'))
import contact_voicing as c
O=R/'out/fsharp-contact';P=R/'bench/fsharp-contact';O.mkdir(exist_ok=True)
BASE=(R/'bench/main-revalidation/effective.toml').read_text()
PARAMS=json.load(open(R/'bench/main-revalidation/note-parameters.json'));p=PARAMS['102'];K=np.arange(1,5);F=p['f0']*K*np.sqrt(1+p['B']*K*K)
V=sorted(set(range(28,128,4))|{45,56,100,127})
BIN=O/'contact_stages';shutil.copy2(R/'target/release/examples/contact_stages',BIN)
def patch(text,fields):
 for key,value in fields.items():
  if '.' in key:
   section,key=key.split('.',1);pattern=rf'(?ms)(^\[{re.escape(section)}\]\n)(.*?)(?=^\[|\Z)';match=re.search(pattern,text);assert match
   block,count=re.subn(rf'^{key} = .*$',f'{key} = {str(value).lower()}',match[2],flags=re.M);assert count==1
   text=text[:match.start(2)]+block+text[match.end(2):]
  else:text,count=re.subn(rf'^{key} = .*$',f'{key} = {str(value).lower()}',text,flags=re.M);assert count==1
 return text
CASES={'base':{},'level':{'unison.height_spread':0},'rigid':{'shank_share':0},'no_core':{'core_hardness':0},'even_hit':{'unison.hit_spread':0},'fixed_hit':{'unison.soft_exponent':0},'no_sym':{'sympathetic.enabled':False},'no_knock':{'noise.action':0},'no_limiter':{'limiter':0}}
# Source of the experimental local voicing, frozen from the previous run.
local=R/'out/main-revalidation/presets/local/n102.toml'
CASES['local']=None

def metrics(name,velocities):
 d=O/'diagnostics'/name;out={};stats=pd.read_csv(d/'stats.csv').set_index('velocity')
 for v in velocities:
  sr,a=wavfile.read(d/f'v{v}.wav');assert sr==48000 and a.shape[1]==4
  force=pd.read_csv(d/f'v{v}-force.csv').iloc[:,1:].to_numpy();t=np.arange(len(force))/sr
  z=np.exp(-2j*np.pi*F[:,None]*t[None,:])@force
  coherent=np.abs(z.sum(axis=1))**2;incoherent=(np.abs(z)**2).sum(axis=1)
  levels=lambda q:(10*np.log10((q[1:]+1e-30)/(q[0]+1e-30))).tolist()
  modes=pd.read_csv(d/f'v{v}-modes.csv');energy=[]
  for k in range(1,5):
   m=modes[modes.k==k];energy.append(np.sum((m.re**2+m.im**2)*(2*np.pi*m.freq)**2))
  out[str(v)]={'audio':c.attack(a[:,:2].astype(float),F),'bridge_force':c.attack(a[:,2].astype(float),F),'bridge_velocity':c.attack(a[:,3].astype(float),F),'force_coherent':levels(coherent),'force_incoherent':levels(incoherent),'cancellation_db':(10*np.log10((coherent+1e-30)/(incoherent+1e-30))).tolist(),'mode_velocity_power':levels(np.array(energy)),**stats.loc[v].to_dict()}
 return out

def main():
 jobs=[]
 for name,fields in CASES.items():
  path=O/'diagnostics'/f'{name}.toml';path.parent.mkdir(parents=True,exist_ok=True);path.write_text(local.read_text() if fields is None else patch(BASE,fields));jobs.append((name,path,V,5))
 for os in [10,20]:jobs.append((f'os{os}',O/'diagnostics/base.toml',[45,56,60,72,100],os))
 def one(job):
  name,path,v,os=job;c.run([BIN,path,O/'diagnostics'/name,102,','.join(map(str,v)),os]);print(name,flush=True);return name,metrics(name,v)
 with ThreadPoolExecutor(4) as ex:results=dict(ex.map(one,jobs))
 c.save(P/'diagnostics.json',results);c.save(P/'design-diagnostics.json',dict(cases=CASES,velocities=V,frequencies=F,contact_substeps=[5,10,20],note=102))
 for name,rows in results.items():
  row=rows['100'];print(name,'P2/P3 audio',row['audio']['partials'][:2],'force',row['force_coherent'][:2],'duration',row['duration_ms'],flush=True)
if __name__=='__main__':main()
