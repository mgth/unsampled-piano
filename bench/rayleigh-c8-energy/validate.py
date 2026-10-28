"""Do8-only final comparison; exact previous support and unchanged F-sharp audio."""
from support import *
import shutil
V=O/'validation';NAMES=['balanced','contact','recovered','recovered_mf'];VV=[45,56,100]
CHOSEN={x['name']:x for x in json.load(open(P/'dense-finalists-results.json'))}
PROFILES={'recovered':CHOSEN['f004'],'recovered_mf':CHOSEN['f005']}
def file(stage,name,n,v):return (a.OLD/stage/'balanced' if name=='balanced' else prev.O/'validation'/stage/'contact' if name=='contact' else V/stage/name)/f'n{n}v{v}.wav'
def scores(rows):
 out={}
 for name in NAMES:
  e={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  out[name]=dict(mae=np.mean([np.mean(np.abs(x)) for x in e.values()]) if e else None,bias=np.mean([np.mean(x) for x in e.values()]) if e else None,rows=len(rows),sources=len(e),low_snr=sum(r['snr'][name]<12 for r in rows))
 return out

def main():
 full=json.load(open(prev.P/'selected.json'))['profiles']['contact'];profiles={name:deepcopy(full) for name in PROFILES}
 for name,x in PROFILES.items():profiles[name]['108']=x['entry']
 c.save(P/'selected.json',dict(profiles=profiles,policy='Full integral radiator; only Do8 contact changes from previous contact profile. F-sharp and every other key frozen. No gain or output EQ. f004 balances level and mf; f005 favors mf. Same references used for tuning and reporting; no default change.'))
 model={name:json.load(open(prev.P/'validation-models.json'))[name] for name in ['balanced','contact']};dense={name:json.load(open(prev.P/'validation-dense.json'))[name] for name in ['balanced','contact']}
 for name,chosen in PROFILES.items():
  m=prev.render(108,name,chosen['entry'],vel=VV,hold=1.1,tail=.1,keep_force=True,root=V/'long');model[name]={};dense[name]={}
  for stage,notes,vels in [('attack',range(93,109),VV),('dense',[97,99,102,108],range(28,128))]:
   table=model if stage=='attack' else dense
   for n in notes:
    for v in vels:
     if n!=108:src=file(stage,'contact',n,v);value=table['contact'][f'{n}:{v}']
     elif stage=='attack':src=V/'long'/name/'n108'/f'n108v{v}.wav';value=m[str(v)]
     else:src=O/'dense'/chosen['name']/'n108'/f'n108v{v}.wav';value=chosen['metrics'][str(v)]
     dst=file(stage,name,n,v);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst);assert src.read_bytes()==dst.read_bytes();table[name][f'{n}:{v}']=value
 c.save(P/'validation-models.json',model);c.save(P/'validation-dense.json',dense)
 refs=json.load(open(R/'bench/main-revalidation/references.json'));rows=c.fixed_rows({**model,'base':json.load(open(R/'bench/main-revalidation/models.json'))['base']},refs);old=json.load(open(prev.P/'validation-summary.json'));ids=lambda rr:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rr};assert ids(rows)==ids(old['attack_rows']) and len(rows)==459
 sus={name:{} for name in NAMES}
 for name in NAMES:
  for n in [96,97,99,102,105,108]:
   par=PAR[str(n)];k=np.arange(1,5);freq=par['f0']*k*np.sqrt(1+par['B']*k*k)
   for v in VV:sus[name][f'{n}:{v}']=c.attack(c.b.load(str(file('attack',name,n,v)),stereo=True),freq,.5,.5)
 srows=[]
 for r in old['sustain_rows']:
  x={k:r[k] for k in ['source','note','velocity','k','target']};key=f"{x['note']}:{x['velocity']}";i=x['k']-2;x['values']={name:sus[name][key]['partials'][i] for name in NAMES};x['snr']={name:sus[name][key]['snr'][i] for name in NAMES};srows.append(x)
 assert ids(srows)==ids(old['sustain_rows']) and len(srows)==71
 dips={};level_dips={};band_changes={}
 for name in NAMES:
  dips[name]={};level_dips[name]={}
  for n in [97,99,102,108]:
   dips[name][n]=[]
   for i in range(3):
    y=[dense[name][f'{n}:{v}']['partials'][i] for v in range(28,128)];dips[name][n].append(float(max(np.maximum.accumulate(y)-y)) if all(z is not None for z in y) else None)
   lev=[prev.measure(file('dense',name,n,v),n)['level_preroll'] for v in range(28,128)];level_dips[name][n]=float(max(np.maximum.accumulate(lev)-lev))
  band_changes[name]={str(v):{key:prev.measure(file('attack',name,108,v),108)[key]-BASELINES['balanced'][str(v)][key] for key in ['p1','level_preroll']}|{'1-2k':prev.measure(file('attack',name,108,v),108)['bands']['1-2k']-BASELINES['balanced'][str(v)]['bands']['1-2k']} for v in VV}
 out=dict(attack=scores([r for r in rows if r['note']>=96]),transition=scores([r for r in rows if r['note']<96]),c8=scores([r for r in rows if r['note']==108]),c8_dynamics={v:scores([r for r in rows if r['note']==108 and r['velocity']==v]) for v in VV},fsharp=scores([r for r in rows if r['note']==102]),sustain=scores(srows),c8_sustain=scores([r for r in srows if r['note']==108]),dips=dips,level_dips=level_dips,band_changes=band_changes,attack_rows=rows,sustain_rows=srows)
 c.save(P/'validation-summary.json',out);print(json.dumps(c.clean({k:v for k,v in out.items() if k not in ['attack_rows','sustain_rows']}),indent=1))
if __name__=='__main__':main()
