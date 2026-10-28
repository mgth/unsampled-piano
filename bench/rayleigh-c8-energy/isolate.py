from support import *

def main():
 jobs=[dict(name='control',entry=SEED)]
 for field,values in dict(q=[.5,.65,.8,1.],h=[.1,.3,1.],p=[-1.,-.5,0.],d=[0.,10e-6,20e-6,40e-6]).items():
  for val in values:
   e=deepcopy(SEED);e['params'][field]=val;jobs.append(dict(name=f'{field}{val:g}',entry=e))
 for field,values in dict(shank_share=[0.,.15,.3,.6],shank_freq=[150,600],shank_loss=[.1,.5,1.,4.],hammer_mass_scale=[.7,1.3,1.6],strike_ratio=[.08,.095,.105,.14]).items():
  for val in values:
   e=deepcopy(SEED);e['extra'][field]=val;jobs.append(dict(name=f'{field}{val:g}',entry=e))
 run_jobs(jobs,'isolate')
if __name__=='__main__':main()
