"""Local F#7 contact study on the frozen upstream engine, close/studio listening.
No changes to frame, unisons, radiation kernels, production defaults or active UI.
"""
from pathlib import Path
import sys,json,importlib.util,tomllib,re,subprocess,hashlib
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from scipy.io import wavfile
R=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('upstream_common',R/'bench/upstream-4a00f00/common.py');u=importlib.util.module_from_spec(spec);spec.loader.exec_module(u)
c=u.c;b=u.b;save=u.save;run=u.run
P=R/'bench/fsharp7-upstream';O=R/'out/fsharp7-upstream';SRC=u.O/'close';BIN=u.BIN
PAR=json.load(open(u.P/'note-parameters.json'));OLD=json.load(open(u.P/'summary.json'))
VEL=[45,56,100];NOTES=list(range(93,109));DENSE=[97,99,102,108]

def bundle(name,fields):
 d=O/name;d.mkdir(parents=True,exist_ok=True)
 text=(SRC/'n102.toml').read_text();text=u.s.prev.patch(text,fields);(d/'n102.toml').write_text(text)
 # Only F#7 contact changes. All other model fields and all filters remain frozen.
 changes=u.changes(tomllib.loads((SRC/'n102.toml').read_text()),tomllib.loads(text))
 assert set(changes)<=set(fields)
 assert not any(k.startswith(('unison.','frame.','room.','soundboard.')) for k in changes)
 t=(SRC/'bundle.toml').read_text()
 for file in ['base.toml','responses.f64']+[f'n{n}.toml' for n in NOTES if n!=102]:t=t.replace('"'+file+'"','"'+str(SRC/file)+'"')
 t=t.replace('Upstream 4a00f00 · Rayleigh intégral',f'F#7 {name} · Upstream 4a00f00 · Rayleigh intégral')
 (d/'bundle.toml').write_text(t);return d/'bundle.toml'

def metric(p,n):
 sr,x=wavfile.read(p);assert sr==48000 and x.shape[1]==2 and np.isfinite(x).all()
 m=c.measure_model(p,PAR[str(n)]);m['bands']=u.s.prev.bands(x);m['level_preroll']=float(b.onset_level(np.pad(x,((1024,0),(0,0)))))
 return m

def render(name,fields,stage='screen',notes=[102],vel=VEL,hold=1.1,tail=.1,prepared=False):
 d=O/name/stage;d.mkdir(parents=True,exist_ok=True);manifest=(O/name/'bundle.toml') if prepared else bundle(name,fields)
 try:run([BIN/'rayleigh_live',manifest,d,','.join(map(str,notes)),','.join(map(str,vel)),hold,tail,512,1])
 except subprocess.CalledProcessError as error:raise RuntimeError(f'{name}/{stage}: {error.stderr}') from error
 m={f'{n}:{v}':metric(d/f'n{n}v{v}.wav',n) for n in notes for v in vel};save(d/'metrics.json',m)
 return m

def scores(m,velocity=None,stage='attack'):
 rows=[r for r in OLD[stage+'_rows'] if r['note']==102 and (velocity is None or r['velocity']==velocity)]
 by={src:[m[f"102:{r['velocity']}"]['partials'][r['k']-2]-r['target'] for r in rows if r['source']==src] for src in {r['source'] for r in rows}}
 return dict(mae=float(np.mean([np.mean(np.abs(e)) for e in by.values()])),bias=float(np.mean([np.mean(e) for e in by.values()])),rows=len(rows))

def summary(m,base):
 result=dict(scores={str(v):scores(m,v) for v in [None,*VEL]})
 targets=[r['target'] for r in OLD['attack_rows'] if (r['note'],r['velocity'],r['k'])==(102,100,3)]
 result['ff_p3_bias']=float(m['102:100']['partials'][1]-np.mean(targets))
 result['delta']={str(v):dict(partials=(np.array(m[f'102:{v}']['partials'],float)-np.array(base[f'102:{v}']['partials'],float)).tolist(),p1=m[f'102:{v}']['p1']-base[f'102:{v}']['p1'],level=m[f'102:{v}']['level_preroll']-base[f'102:{v}']['level_preroll'],band_1_2k=m[f'102:{v}']['bands']['1-2k']-base[f'102:{v}']['bands']['1-2k']) for v in VEL}
 return result

def variants():
 d={'control':{}}
 for key,values in {'hardness':[.24,.27,.33,.36], 'hysteresis':[.2,.25,.35,.4], 'hammer_mass_scale':[.9,.95,1.05,1.1], 'strike_ratio':[.095,.10,.11,.115], 'shank_freq':[200,250,350,400], 'shank_loss':[1,1.5,2.5,3], 'contact_width_scale':[.8,.9,1.1,1.2], 'core_hardness':[0,10,50]}.items():
  for v in values:d[f'{key}_{v}']={key:v}
 for p in [-.5,-.25,.25,.5]:d[f'exponent_{p}']={'p_offset':p,'hardness':.3*.2**(-p)}
 return d

def screen(vs,stage='screen'):
 save(P/f'{stage}-variants.json',vs)
 def job(item):
  name,fields=item;m=render(name,fields,stage);print(stage,name,flush=True);return name,m
 with ThreadPoolExecutor(4) as ex:models=dict(ex.map(job,vs.items()))
 base=models.get('control') or json.load(open(O/'control/screen/metrics.json'))
 summaries={name:summary(m,base) for name,m in models.items()}
 save(P/f'{stage}-models.json',models);save(P/f'{stage}-summary.json',summaries)
 for name,s in sorted(summaries.items(),key=lambda x:abs(x[1]['ff_p3_bias']))[:15]:print(name,json.dumps(c.clean(s)),flush=True)

if __name__=='__main__':
 screen(variants() if len(sys.argv)<2 else json.load(open(sys.argv[1])), 'screen' if len(sys.argv)<3 else sys.argv[2])
