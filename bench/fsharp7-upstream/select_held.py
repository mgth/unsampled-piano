from study import *
from sustain_screen import held

base=json.load(open(P/'screen-summary.json'))['control'];susbase=held('control','screen')
result={}
for stage in ['screen','refine','factorial','held']:
 for name,s in json.load(open(P/f'{stage}-summary.json')).items():
  m=held(name,stage);delta={str(v):m[f'102:{v}']['partials'][0]-susbase[f'102:{v}']['partials'][0] for v in VEL}
  failures=[]
  for v in ['45','56']:
   if s['scores'][v]['mae']>base['scores'][v]['mae']+.05:failures.append('attack_mae_'+v)
   if s['delta'][v]['level']<-.25:failures.append('level_'+v)
   if s['delta'][v]['band_1_2k']<-.25:failures.append('band_'+v)
   if delta[v]>.05:failures.append('held_p2_'+v)
  if s['scores']['None']['mae']>base['scores']['None']['mae']+.05:failures.append('all_mae')
  if abs(s['ff_p3_bias'])>=abs(base['ff_p3_bias'])-.5:failures.append('ff_p3')
  result[name]=dict(stage=stage,summary=s,held_delta_p2=delta,failures=failures)
save(P/'held-selection.json',result)
for name,s in sorted(result.items(),key=lambda x:(len(x[1]['failures']),abs(x[1]['summary']['ff_p3_bias'])))[:25]:
 print(name,'ffP3',round(s['summary']['ff_p3_bias'],3),'held', {k:round(v,3) for k,v in s['held_delta_p2'].items()},'fail',s['failures'])
