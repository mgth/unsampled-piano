"""Two-position, contact-only neighborhood; identical velocities and close refs."""
from support import *
from concurrent.futures import ThreadPoolExecutor

def design():
 jobs=[dict(name='control',entry=ENTRIES['recovered'])]
 for field,values in dict(q=[.25,.35],p=[-1.25,-.75],h=[.4,.8],w=[.8,1.2],d=[10e-6,20e-6]).items():
  for value in values:
   e=deepcopy(ENTRIES['recovered']);e['params'][field]=value;jobs.append(dict(name=f'{field}{value:g}',entry=e))
 for field,values in dict(shank_share=[.4,.5],shank_freq=[250,350,400],shank_loss=[.025,.075,.1],strike_ratio=[.11,.115,.125,.13],hammer_mass_scale=[.9,1.1]).items():
  for value in values:
   e=deepcopy(ENTRIES['recovered']);e['extra'][field]=value;jobs.append(dict(name=f'{field}{value:g}',entry=e))
 return jobs

def main():
 geom=sys.argv[1];kernel(geom);jobs=design();c.save(P/f'{geom}-screen-design.json',dict(cases=jobs,velocities=energy.VEL,policy='Same Do8-only parameter changes at two listening positions. Close-mic references stay frozen; no player-reference score inferred.'))
 def one(job):
  m=capture(job['entry'],job['name'],geom,vel=energy.VEL,root=O/'screen',keep=False);s=prev.score(108,m) if geom=='close' else {};print(geom,job['name'],s.get('mae'),flush=True);return dict(**job,metrics=m,**s)
 with ThreadPoolExecutor(4) as ex:out=list(ex.map(one,jobs))
 c.save(P/f'{geom}-screen.json',out)
if __name__=='__main__':main()
