"""Contact-only studies on the full causal radiator. No output EQ/crossfade."""
from pathlib import Path
import sys,json,shutil,tomllib
import numpy as np
from scipy.io import wavfile
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));sys.path.insert(0,str(R/'bench/rayleigh-audio'));sys.path.insert(0,str(R/'bench/bridge-ear'))
import audio as a
from preset_fields import patch
c=a.c;P=R/'bench/rayleigh-contact';O=R/'out/rayleigh-contact';BASE=(R/'bench/main-revalidation/effective.toml').read_text();PAR=a.PAR
OLD=json.load(open(R/'bench/rayleigh-audio/validation-summary.json'));ROWS=OLD['attack_rows'];BAL=json.load(open(R/'bench/fsharp-contact/selected.json'))['profiles']['balanced'];LOCAL=json.load(open(R/'bench/contact-residuals/profile-local.json'))['notes'];VEL=sorted(set(range(28,128,8))|{45,56,100,127})

def entry(n):return BAL if n==102 else {'params':c.DEFAULT,'extra':{}} if n in [97,99] else LOCAL[str(n)]
def preset(e,path):
 c.write_preset(path,BASE,**e['params']);path.write_text(patch(path.read_text(),e.get('extra',{})));return path

def bands(x):
 on=c.b.onset(x);seg=x[on:on+2880];w=np.hanning(len(seg));X=np.fft.rfft(seg*w[:,None],n=16384,axis=0);f=np.fft.rfftfreq(16384,1/48000);psd=2*abs(X)**2/(48000*np.sum(w*w));power=psd.mean(axis=1)*48000/16384
 return {name:float(10*np.log10(np.sum(power[(f>=lo)&(f<hi)])+1e-30)) for name,lo,hi in [('1-2k',1000,2000),('2-4k',2000,4000),('4-8k',4000,8000),('8-12k',8000,12000)]}

def measure(path,n):
 x=c.b.load(str(path),stereo=True);m=c.measure_model(path,PAR[str(n)]);m['bands']=bands(x);m['level_preroll']=float(c.b.onset_level(np.pad(x,((1024,0),(0,0)))));return m

def render(n,name,e,vel=VEL,hold=.2,tail=.05,keep_force=False,root=None):
 root=O/'screen' if root is None else root;d=root/name/f'n{n}';p=preset(e,d/'preset.toml');capture=d/'capture';a.c.run([a.BIN,'capture',p,capture,n,','.join(map(str,vel)),hold,tail]);metrics={};checks={}
 for v in vel:
  dry,new,check=a.stages(capture,n,v);assert check['rms_error_db'] < -90
  path=d/f'n{n}v{v}.wav';a.save_audio(p,new,path);metrics[str(v)]=measure(path,n);checks[str(v)]=check
  if not keep_force:(capture/f'n{n}v{v}.force.f32').unlink()
 c.save(d/'metrics.json',metrics);c.save(d/'checks.json',checks);return metrics

def score(n,m):
 rr=[r for r in ROWS if r['note']==n];errors={s:[] for s in {r['source'] for r in rr}}
 for r in rr:errors[r['source']].append(m[str(r['velocity'])]['partials'][r['k']-2]-r['target'])
 signed={};pervel={}
 for v in [45,56,100]:
  signed[v]=[]
  for k in [2,3,4]:
   target=[r['target'] for r in rr if r['velocity']==v and r['k']==k]
   signed[v].append(m[str(v)]['partials'][k-2]-np.mean(target) if target else None)
  source={s:[abs(m[str(v)]['partials'][r['k']-2]-r['target']) for r in rr if r['source']==s and r['velocity']==v] for s in errors};pervel[v]=np.mean([np.mean(x) for x in source.values() if x])
 vs=sorted(map(int,m));dips=[]
 for i in range(3):
  y=[m[str(v)]['partials'][i] for v in vs];dips.append(float(max(np.maximum.accumulate(y)-y)) if all(x is not None for x in y) else None)
 level=[m[str(v)]['level_preroll'] for v in vs]
 return dict(mae=np.mean([np.mean(np.abs(e)) for e in errors.values()]),rows=len(rr),by_velocity=pervel,signed=signed,dips=dips,level_dip=float(max(np.maximum.accumulate(level)-level)))
