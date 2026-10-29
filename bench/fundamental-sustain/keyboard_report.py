"""Verified 88-key outcomes, static comparisons, and compact interactive data."""
import keyboard_common as c
import keyboard_diagnose as d
import keyboard_select as select
import json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
FR=['Do','Do♯','Ré','Ré♯','Mi','Fa','Fa♯','Sol','Sol♯','La','La♯','Si']
def name(n):return FR[n%12]+str(n//12-1)

def main():
 base=json.loads((c.O/'current.json').read_text());candidate=json.loads((c.O/'candidate.json').read_text());refs=json.loads((c.O/'references.json').read_text())['notes'];selection=json.loads((c.O/'selection.json').read_text());validation=json.loads((c.O/'validation.json').read_text());diagnosis=json.loads((c.O/'diagnosis.json').read_text())
 changed=set(map(int,selection['selected']));summary={};data={};after={}
 for n in c.NOTES:
  for v in c.VELOCITIES:
   k=f'{n}|{v}';before=diagnosis['rows'][k];a=d.assess(n,candidate[k],refs[k]);after[k]=a
   def h1(row):return [z[0] for z in row['decay_self'][:7]]
   raw={nm:h1(row) for nm,row in refs[k].items() if nm!='main'}
   data[k]=dict(current=h1(base[k]),candidate=h1(candidate[k]),main=h1(refs[k]['main']),refs=raw,before=before['status'],after=a['status'],reference_count=a['reference_count'],excluded=a['excluded'],model_issue=a['model_issue'],anchor=a['anchor'])
  if n in changed:
   assert select.guards(n,candidate,refs,base) is None,(n,select.guards(n,candidate,refs,base))
   old=select.error(n,base,refs,base);new=select.error(n,candidate,refs,base)
   assert new<old*.7,(n,old,new)
   summary[n]=dict(name=name(n),fields=selection['selected'][str(n)],score_before=old,score_after=new,status_before=[diagnosis['rows'][f'{n}|{v}']['status'] for v in c.VELOCITIES],status_after=[after[f'{n}|{v}']['status'] for v in c.VELOCITIES])
 statuses=['fast','slow','mixed','within','insufficient']
 counts=dict(before={st:sum(x['status']==st for x in diagnosis['rows'].values()) for st in statuses},after={st:sum(x['status']==st for x in after.values()) for st in statuses})
 controls=[z for k,z in validation['checks'].items() if int(k.split('|')[0]) not in changed]
 assert max(z['audible_partial_max_delta'] for z in controls)<.3
 report=dict(scope='88 keys × 3 velocities; same close/studio experimental engine',counts=counts,selected=summary,control_renders=len(controls),max_control_partial_delta=max(z['audible_partial_max_delta'] for z in controls),diagnosis_before=diagnosis['rows'],diagnosis_after=after,validation=validation,selection=selection)
 report['inputs']=dict(current_render=json.loads((c.O/'current_21/render.json').read_text()),main_commit=json.loads((c.R/'out/bench/harmonic_local/result.json').read_text())['commit'],reference_audio={k:{nm:r['sha256'] for nm,r in rows.items()} for k,rows in refs.items()})
 c.save(c.R/'bench/fundamental-sustain/keyboard_results.json',report)
 c.save(c.O/'visual-data.json',dict(notes={n:name(n) for n in c.NOTES},data=data,counts=counts,selected=sorted(changed)))
 # Raw reference mean and range are retained, including suspicious attack anchors.
 fig,axes=plt.subplots(3,2,figsize=(16,10),sharex=True)
 for ri,v in enumerate(c.VELOCITIES):
  for ci,(ti,t) in enumerate([(3,.5),(6,1.1)]):
   ax=axes[ri,ci];lo=[];hi=[];ys={x:[] for x in ['main','current','candidate']}
   for n in c.NOTES:
    row=data[f'{n}|{v}'];values=[a[ti] for a in row['refs'].values() if a[ti] is not None]
    mean=np.mean(values) if values else np.nan
    lo.append(min(values)-mean if values else np.nan);hi.append(max(values)-mean if values else np.nan)
    for key in ys:ys[key].append(row[key][ti]-mean if row[key][ti] is not None else np.nan)
   ax.fill_between(c.NOTES,lo,hi,color='#777777',alpha=.18,label='Min–max de toutes les références lisibles')
   ax.axhline(0,color='#555555',lw=.8,ls=':',label='Moyenne des références')
   for key,label,color,ls in [('main','Main','#9b7099','--'),('current','App actuelle · plateau réduit','#25834b','-'),('candidate','Candidat clavier','#c13d43','-')]:
    ax.plot(c.NOTES,ys[key],color=color,ls=ls,lw=1.2,label=label)
   ax.set_title(f'Vélocité {v} · {str(t).replace(".",",")} s');ax.grid(alpha=.15);ax.set_ylabel('Écart de tenue H1 à la moyenne (dB)')
   ax.set_xticks([21]+list(range(36,109,12)),[name(n) for n in [21]+list(range(36,109,12))]);ax.set_xlim(20,109)
 handles,labels=axes[0,0].get_legend_handles_labels();fig.legend(handles,labels,loc='upper center',ncol=3,bbox_to_anchor=(.5,.965),frameon=False)
 fig.suptitle('Fondamentale · 88 touches, trois vélocités',fontsize=17);fig.tight_layout(rect=[0,0,1,.90]);fig.savefig(c.R/'docs/img/fundamental_sustain_keyboard.png',dpi=140);plt.close(fig)
 # Examples include both accepted improvements and unresolved strong alerts.
 examples=[57,38,60,80,77,104]
 fig,axes=plt.subplots(2,3,figsize=(15,8),sharex=True)
 for ax,n in zip(axes.flat,examples):
  v=56;k=f'{n}|{v}';row=data[k];arr=np.array(list(row['refs'].values()),float)
  ax.fill_between(c.TIMES,np.nanmin(arr,axis=0),np.nanmax(arr,axis=0),color='#777777',alpha=.18)
  ax.plot(c.TIMES,np.nanmean(arr,axis=0),color='#555555',ls=':',label='Moyenne des vrais pianos')
  for key,label,color,ls in [('main','Main','#9b7099','--'),('current','App actuelle','#25834b','-'),('candidate','Candidat clavier','#c13d43','-')]:ax.plot(c.TIMES,row[key],color=color,ls=ls,lw=1.7,label=label)
  ax.set_title(f'{name(n)} · vélocité {v}'+(' · inchangé' if n not in changed else ''));ax.grid(alpha=.15);ax.set_xlabel('Temps après attaque (s)');ax.set_ylabel('H1 / son attaque (dB)');ax.set_xticks([0,.3,.5,.7,1.1])
 handles,labels=axes.flat[0].get_legend_handles_labels();fig.legend(handles,labels,loc='upper center',ncol=4,bbox_to_anchor=(.5,.95),frameon=False);fig.suptitle('Chutes rapides et tenues longues · même comparaison',fontsize=17);fig.tight_layout(rect=[0,0,1,.88]);fig.savefig(c.R/'docs/img/fundamental_sustain_keyboard_examples.png',dpi=150);plt.close(fig)
 print(json.dumps(counts,indent=2));print('changed',len(changed),'control renders',len(controls))

if __name__=='__main__':main()
