"""Publish measured and explicitly estimated references on the existing graphs."""
import json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import keyboard_common as c
import adaptive_references as ar
import adaptive_search as search
from adaptive_scan import O,FOCUS
from keyboard_report import name

def main():
 m=json.loads((O/'measurements.json').read_text());rows=m['rows'];ref=json.loads((O/'references.json').read_text());final=json.loads((O/'final.json').read_text());sel=json.loads((O/'selection.json').read_text())
 payload=json.loads((c.O/'visual-data.json').read_text())
 all_trials=json.loads((O/'search.json').read_text())['results']|json.loads((O/'extra.json').read_text())['results']
 old=json.loads((c.O/'candidate.json').read_text());trial_for={}
 for n in map(int,sel['outcomes']):
  if str(n) in sel['selected']:continue
  ranking=sorted((search.score(n,rs,ref['references'],rows),nm) for nm,rs in all_trials.items())
  _,nm=ranking[0];trial_for[n]=dict(name=nm,reason=search.guard(n,all_trials[nm],ref['references'],rows,old) or 'amélioration inférieure à 30 %')
 outcomes={}
 for k,z in payload['data'].items():
  z['candidate']=[a[0] for a in final['legacy'][k]['decay_self'][:7]]
 for n in FOCUS:
  for v in c.VELOCITIES:
   k=f'{n}|{v}';r=ref['references'][k];new=final['adaptive'][k];before=ar.assess(rows[k]['current'],r);after=ar.assess(new,r)
   estimates=list(r['estimated'].values());used=list(r['actual'].values())+[z['curve'] for z in estimates]
   data=dict(times=new['times'],current=rows[k]['current']['curve'],candidate=new['curve'],main=rows[k]['main']['curve'],refs={nm:z['curve'] for nm,z in rows[k].items() if nm not in ar.MODEL_NAMES},before=before['status'],after=after['status'],reference_count=r['count'],measured_count=len(r['actual']),estimated_count=len(estimates),model_issue='références fiables insuffisantes' if after['status']=='insufficient' else None,anchor=0,anchor_time=new['anchor'],window_duration=new['duration'])
   if estimates:
    data.update(estMean=np.mean([z['curve'] for z in estimates],axis=0).tolist(),estLo=np.min([z['lo'] for z in estimates],axis=0).tolist(),estHi=np.max([z['hi'] for z in estimates],axis=0).tolist(),comparisonMean=np.mean(used,axis=0).tolist(),neighbour_label='Voisins : '+ '; '.join(nm+' '+ '/'.join(name(m) for m in z['neighbours']) for nm,z in r['estimated'].items()))
   if n in trial_for:
    trial=trial_for[n];data.update(trial=all_trials[trial['name']][k]['adaptive']['curve'],trial_reason=trial['reason'])
   payload['data'][k]=data
   outcomes[k]=dict(before=before,after=after,previous=ar.assess(rows[k]['previous'],r),reference=r,current=rows[k]['current'],candidate=new,main=rows[k]['main'])
 old_diagnosis=json.loads((c.O/'diagnosis.json').read_text())['rows']
 formerly=[k for k,z in old_diagnosis.items() if z['status']=='insufficient']
 resolved=[k for k in formerly if outcomes[k]['after']['status']!='insufficient']
 counts=dict(previous_insufficient=len(formerly),now_evaluable=len(resolved),remaining_insufficient=len(formerly)-len(resolved),resolved=resolved,new_corrected_keys=final['changed'],control_count=final['control_count'],max_control_delta=final['max_control_delta'])
 result=dict(counts=counts,method=m['method'],calibration=ref['calibration'],outcomes=outcomes,selection=sel,trials_not_retained=trial_for,bundle=final['bundle'],inputs={k:{nm:dict(path=z['path'],sha256=z['sha256']) for nm,z in rs.items()} for k,rs in rows.items()})
 c.save(c.R/'bench/fundamental-sustain/adaptive_results.json',result)
 payload['adaptive_counts']=counts;c.save(O/'visual-data.json',payload)
 fig,axes=plt.subplots(3,3,figsize=(15,12))
 for ax,n in zip(axes.flat,FOCUS):
  z=payload['data'][f'{n}|100'];ts=z['times'];matrix=np.array(list(z['refs'].values()),float)
  if np.isfinite(matrix).any():
   ax.fill_between(ts,np.nanmin(matrix,axis=0),np.nanmax(matrix,axis=0),color='#777777',alpha=.18,label='Étendue mesurée')
   ax.plot(ts,np.nanmean(matrix,axis=0),color='#555555',ls=':',label='Moyenne mesurée')
  if z.get('estMean'):
   ax.fill_between(ts,z['estLo'],z['estHi'],color='#2785a4',alpha=.12,label='Voisins + marge')
   ax.plot(ts,z['estMean'],color='#2785a4',ls='--',label='Voisins estimés')
  for key,label,color,style in [('main','Main','#9b7099','--'),('current','App actuelle','#25834b','-'),('candidate','Candidat retenu','#c13d43','-'),('trial','Essai non retenu','#bd7a24',':')]:
   if key in z:ax.plot(ts,z[key],label=label,color=color,ls=style,lw=1.8)
  ax.set_title(name(n)+' · vélocité 100'+(' · estimé' if z['estimated_count'] else ''));ax.grid(alpha=.15)
  ax.set_xlabel('Temps après attaque (s)');ax.set_ylabel('H1 / '+('0,25 s' if z['anchor_time']==.25 else 'attaque')+' (dB)')
  ax.set_xticks(ts if len(ts)==4 else [.03,.25,.5,.75,1.1])
 handles={}
 for ax in axes.flat:
  hs,ls=ax.get_legend_handles_labels();handles.update(zip(ls,hs))
 fig.legend(handles.values(),handles.keys(),loc='upper center',ncol=4,bbox_to_anchor=(.5,.965),frameon=False)
 fig.suptitle('H1 · mesure adaptée et références voisines explicites',fontsize=17);fig.tight_layout(rect=[0,0,1,.91]);fig.savefig(c.R/'docs/img/fundamental_sustain_adaptive.png',dpi=140);plt.close(fig)
 print(json.dumps(counts,indent=2));
 for n in final['changed']:
  for v in c.VELOCITIES:
   k=f'{n}|{v}';a=outcomes[k];print(k,'before',a['before']['value'],'after',a['after']['value'],a['after']['status'])

if __name__=='__main__':main()
