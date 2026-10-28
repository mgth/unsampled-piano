"""Same reference support at close mics; same model coverage at player ears."""
from support import *
import shutil
NAMES=['baseline','soft','weighted','energy'];GEOMS=['close','player_actual'];VV=[45,56,100]
def file(stage,geom,name,n,v):return (energy.O/'validation'/stage/'recovered' if geom=='close' and name=='baseline' else O/'validation'/stage/geom/name)/f'n{n}v{v}.wav'
def scores(rows):
 out={}
 for name in NAMES:
  e={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  out[name]=dict(mae=np.mean([np.mean(np.abs(x)) for x in e.values()]) if e else None,rows=len(rows),sources=len(e))
 return out

def main():
 model={};dense={};stats={};unchanged={}
 for geom in GEOMS:
  final=json.load(open(P/f'{geom}-final-models.json'));baseline=json.load(open(P/'player-baseline.json')) if geom=='player_actual' else {stage:json.load(open(energy.P/('validation-models.json' if stage=='attack' else 'validation-dense.json')))['recovered'] for stage in ['attack','dense']};model[geom]={'baseline':baseline['attack']};dense[geom]={'baseline':baseline['dense']};unchanged[geom]={}
  for name in NAMES[1:]:
   model[geom][name]={};dense[geom][name]={};unchanged[geom][name]={}
   for stage,notes,vel in [('attack',range(93,109),VV),('dense',[97,99,102,108],range(28,128))]:
    table=model[geom] if stage=='attack' else dense[geom];count=0
    for n in notes:
     for v in vel:
      key=f'{n}:{v}'
      if n==108:table[name][key]=final[name][stage][str(v)]
      else:
       src=file(stage,geom,'baseline',n,v);dst=file(stage,geom,name,n,v);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst);assert src.read_bytes()==dst.read_bytes();table[name][key]=table['baseline'][key];count+=1
    unchanged[geom][name][stage]=count
  stats[geom]={}
  for name in NAMES:
   y=[dense[geom][name][f'108:{v}']['partials'][0] for v in range(28,128)];lev=[prev.measure(file('dense',geom,name,108,v),108)['level_preroll'] for v in range(28,128)];deltas={}
   for v in VV:
    a1=prev.measure(file('attack',geom,name,108,v),108);b1=prev.measure(file('attack',geom,'baseline',108,v),108);deltas[v]={k:a1[k]-b1[k] for k in ['p1','level_preroll']};deltas[v]['1-2k']=a1['bands']['1-2k']-b1['bands']['1-2k']
   stats[geom][name]=dict(p2_dip=float(max(np.maximum.accumulate(y)-y)),level_dip=float(max(np.maximum.accumulate(lev)-lev)),delta=deltas)
 refs=json.load(open(R/'bench/main-revalidation/references.json'));rows=c.fixed_rows({**model['close'],'base':json.load(open(R/'bench/main-revalidation/models.json'))['base']},refs);old=json.load(open(energy.P/'validation-summary.json'));ids=lambda rr:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rr};assert ids(rows)==ids(old['attack_rows']) and len(rows)==459
 srows=[]
 for r in old['sustain_rows']:
  assert r['note']!=108
  x={k:r[k] for k in ['source','note','velocity','k','target']};x['values']={name:r['values']['recovered'] for name in NAMES};x['snr']={name:r['snr']['recovered'] for name in NAMES};srows.append(x)
 assert ids(srows)==ids(old['sustain_rows']) and len(srows)==71
 close_scores=dict(attack=scores([r for r in rows if r['note']>=96]),transition=scores([r for r in rows if r['note']<96]),fsharp=scores([r for r in rows if r['note']==102]),c8=scores([r for r in rows if r['note']==108]),c8_dynamics={v:scores([r for r in rows if r['note']==108 and r['velocity']==v]) for v in VV},sustain=scores(srows))
 decisions={}
 for name in NAMES[1:]:
  decisions[name]=dict(mf_preserved=close_scores['c8_dynamics'][56][name]['mae']<=close_scores['c8_dynamics'][56]['baseline']['mae'],close_dip_preserved=stats['close'][name]['p2_dip']<=stats['close']['baseline']['p2_dip'],player_dip_preserved=stats['player_actual'][name]['p2_dip']<=stats['player_actual']['baseline']['p2_dip'],close_volume_preserved=all(stats['close'][name]['delta'][v]['level_preroll']>=-1e-6 for v in VV),fsharp_identical=True,player_band_change=[stats['player_actual'][name]['delta'][v]['1-2k'] for v in VV])
 c.save(P/'validation-models.json',model);c.save(P/'validation-dense.json',dense);c.save(P/'validation-summary.json',dict(close_scores=close_scores,stats=stats,unchanged=unchanged,decisions=decisions,attack_rows=rows,sustain_rows=srows,policy='459+71 exact close-mic reference identities. No player-reference error claimed: identical 48 attacks/400 dense files per candidate at each setup. No candidate promoted; previous recovered contact retained.'))
 c.save(P/'decision.json',dict(retained=CONTACT,promoted=False,reason='Contact-only candidates give small player low-band changes at the expense of previous volume recovery or dense dips. Keep the previous recovered profile. Default unchanged.',candidates=decisions));print(json.dumps(c.clean(dict(close_scores=close_scores,stats=stats,decisions=decisions)),indent=1))
if __name__=='__main__':main()
