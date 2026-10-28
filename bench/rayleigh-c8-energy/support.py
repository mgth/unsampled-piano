"""Do8-only continuation of the frozen integral-radiation experiment."""
from pathlib import Path
import importlib.util,sys,json
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('previous_contact',ROOT/'bench/rayleigh-contact/common.py');prev=importlib.util.module_from_spec(spec);spec.loader.exec_module(prev)
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from scipy.io import wavfile
from copy import deepcopy
R=ROOT;P=R/'bench/rayleigh-c8-energy';O=R/'out/rayleigh-c8-energy';c=prev.c;a=prev.a;PAR=prev.PAR;VEL=prev.VEL
SEED=json.load(open(prev.P/'selected.json'))['profiles']['contact']['108']
BASELINES={name:{str(v):prev.measure((a.OLD/'attack/balanced' if name=='balanced' else prev.O/'validation/attack/contact')/f'n108v{v}.wav',108) for v in [45,56,100]} for name in ['balanced','contact']}
def evaluate(m):
 s=prev.score(108,m)
 s['delta']={name:{str(v):{key:m[str(v)][key]-base[str(v)][key] for key in ['p1','level_preroll']}|{'1-2k':m[str(v)]['bands']['1-2k']-base[str(v)]['bands']['1-2k']} for v in [45,56,100]} for name,base in BASELINES.items()}
 return s

def run_jobs(jobs,filename,vel=VEL,root=None,hold=.2,tail=.05,keep=False):
 c.save(P/(filename+'-design.json'),dict(cases=jobs,velocities=list(vel),policy='Frozen stereo/Plath benchmark. Only Do8 contact fields vary; full integral radiation, no gain fit or EQ.'))
 def one(x):
  m=prev.render(108,x['name'],x['entry'],vel=vel,root=O/filename if root is None else root,hold=hold,tail=tail,keep_force=keep);s=evaluate(m);print(x['name'],c.clean({k:s[k] for k in ['mae','by_velocity','dips','delta']}),flush=True);return dict(**x,metrics=m,**s)
 with ThreadPoolExecutor(4) as ex:results=list(ex.map(one,jobs))
 c.save(P/(filename+'-results.json'),results);return results
