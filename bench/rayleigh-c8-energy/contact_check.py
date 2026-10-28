"""Check the actual handed-off modal amplitudes at 5/10/20 contact substeps."""
from support import *

def main():
 choices=json.load(open(P/'selected.json'))['profiles'];entries={'contact':SEED,**{name:ps['108'] for name,ps in choices.items()}};jobs=[]
 for name,e in entries.items():
  p=prev.preset(e,O/'mechanics'/name/'preset.toml')
  for os in [5,10,20]:jobs.append((name,os,p))
 def one(job):
  name,os,p=job;d=p.parent/f'os{os}';c.run([prev.O/'bin/contact_stages',p,d,108,'32,45,56,80,100,127',os]);stats=np.genfromtxt(d/'stats.csv',delimiter=',',names=True);out={}
  for row in stats:
   v=int(row['velocity']);m=np.genfromtxt(d/f'v{v}-modes.csv',delimiter=',',names=True);power=np.array([sum((m['re']**2+m['im']**2)[m['k']==k]) for k in [1,2]]);out[v]=dict(stats={k:float(row[k]) for k in stats.dtype.names},modal_p2=10*np.log10(power[1]/power[0]))
  return f'{name}:{os}',out
 with ThreadPoolExecutor(4) as ex:data=dict(ex.map(one,jobs))
 delta={name:{os:{v:data[f'{name}:{os}'][v]['modal_p2']-data[f'{name}:20'][v]['modal_p2'] for v in data[f'{name}:{os}']} for os in [5,10]} for name in entries};c.save(P/'contact-convergence.json',dict(data=data,delta_vs_20=delta,policy='Isolated modal power ratio; no claim that production audio uses 20 substeps. Includes dense-range endpoints.'))
 print(json.dumps(c.clean(delta),indent=1))
if __name__=='__main__':main()
