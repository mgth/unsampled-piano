"""Fixed-coverage checks and graphs for the plateau candidate."""
from pathlib import Path
import json,hashlib
import numpy as np
from scipy.io import wavfile
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
R=Path(__file__).resolve().parents[2];P=Path(__file__).resolve().parent;O=R/'out/bench/fundamental-sustain'
x=json.loads((O/'plateau_validation.json').read_text());r=x['results'];ref=json.loads((R/'out/bench/harmonic_local/harmonic_diagnostic.json').read_text())
old=json.loads((O/'prototype.json').read_text())['results']
selected=list(map(int,x['selected']));rows=[];controls={};attack=[];upper=[];early=[];later=[];peak=[];release=[];release_delta=[]
for n in x['notes']:
 for v in x['velocities']:
  k=f'{n}|{v}'
  if n not in selected:
   controls[k]=x['checks'][k];assert controls[k]['identical'],k
   continue
  a=r['plateau_control'][k];b=r['plateau_candidate'][k];aa=np.array(a['levels'],float);bb=np.array(b['levels'],float)
  assert np.isfinite(aa[:7,:8]).all() and np.isfinite(bb[:7,:8]).all(),k
  attack.append(abs(bb[0,0]-aa[0,0]));upper.extend(abs(bb[7,1:]-aa[7,1:]));early.append(b['decay_self'][3][0]-a['decay_self'][3][0]);later.append(b['decay_self'][6][0]-a['decay_self'][6][0])
  sr,w=wavfile.read(O/'plateau_candidate'/f'n{n}v{v}.wav');assert sr==48000 and np.isfinite(w).all();peak.append(float(abs(w).max()))
  rms=lambda p,q:np.sqrt(np.mean(w[int(p*sr):int(q*sr)]**2))
  release.append(float(20*np.log10(rms(4.3,4.5)/rms(3.8,4.0))))
  if n==62:
   sr_control,w_control=wavfile.read(O/'plateau_control'/f'n{n}v{v}.wav');assert sr_control==sr
   control_tail=np.sqrt(np.mean(w_control[int(4.3*sr):int(4.5*sr)]**2))
   release_delta.append(float(20*np.log10(rms(4.3,4.5)/control_tail)))
  references={nm:[s['decay_self'][i][0] for i in [3,6]] for nm,s in ref['notes'][k].items() if nm not in ref['models']}
  rows.append(dict(note=n,velocity=v,previous=[a['decay_self'][i][0] for i in [3,6]],candidate=[b['decay_self'][i][0] for i in [3,6]],absolute_delta=[bb[i,0]-aa[i,0] for i in [3,6]],references=references,late_relative_attack=[z[0]-b['levels'][0][0] for z in b['late_levels']]))
# Preserve the previously auditioned comparison: every available control WAV
# from the preceding run must match, rather than merely have similar metrics.
old_paths=list((O/'prototype_early_test').glob('n*v*.wav'))
assert len(old_paths)==21
for p in old_paths:assert p.read_bytes()==(O/'plateau_control'/p.name).read_bytes(),p.name
assert len(controls)==33
assert max(map(abs,early))<.75
assert max(attack)<.1
assert max(upper)<2
assert max(later)<-4
assert len(release_delta)==3 and max(release_delta)<0
checks=dict(unchanged_controls=len(controls),previous_renders_reproduced=len(old_paths),max_h1_attack_delta=max(attack),h1_half_second_delta_range=[min(early),max(early)],h1_1_1_second_delta_range=[min(later),max(later)],max_upper_partial_sustain_delta=max(upper),peak=max(peak),release_rms_drop_range=[min(release),max(release)])
checks['d4_release_rms_delta_range']=[min(release_delta),max(release_delta)]
result=dict(selected=x['selected'],release_horizontal=x.get('release_horizontal',{}),method='Mean stereo FFT power. H1 relative to its own 0–60 ms peak. Half-second: 0.4–0.6 s; late: 1–1.2 s.',rows=rows,checks=checks,controls=controls,bundle_manifest=x['bundle_manifest'],renderer_sha256=x['renderer_sha256'])
(P/'plateau_results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(checks,indent=2))
times=[.03,.13,.3,.5,.7,.9,1.1]
names={62:'Ré4',64:'Mi4',69:'La4'}
def plot(pairs,title,dest):
 fig,axes=plt.subplots(1,3,figsize=(12,5.2),sharey=True)
 for ax,(n,v) in zip(axes,pairs):
  k=f'{n}|{v}';src=ref['notes'][k]
  refs=np.array([[s['decay_self'][t][0] for t in range(7)] for nm,s in src.items() if nm not in ref['models']],float)
  ax.fill_between(times,np.nanmin(refs,axis=0),np.nanmax(refs,axis=0),color='#777777',alpha=.17,label='Min–max des vrais pianos')
  ax.plot(times,np.nanmean(refs,axis=0),color='#555555',linestyle=':',lw=1.5,label='Moyenne des vrais pianos')
  for label,s,c,ls,lw in [
   ('Main',src['model'],'#9b7099','--',1),
   ('Prototype initial',old['prototype_base'][k],'#bf7626',':',1.2),
   ('Premier essai · h = 0,1',old['prototype_test'][k],'#2678b7','--',1.3),
   ('Réglage actuel · h = 0,3',r['plateau_control'][k],'#25834b','-',1.8),
   ('Candidat · plateau réduit',r['plateau_candidate'][k],'#c13d43','-',2.4)]:
   ax.plot(times,[s['decay_self'][t][0] for t in range(7)],color=c,ls=ls,lw=lw,label=label)
  ax.axvline(.5,color='#888888',lw=.8,alpha=.4)
  vals=r['plateau_candidate'][k]['decay_self'];y0=vals[3][0];y1=vals[6][0]
  ax.scatter([.5,1.1],[y0,y1],color='#c13d43',s=20,zorder=5)
  ax.annotate(f'{y0:.1f} dB'.replace('.',','),(.5,y0),xytext=(5,12),textcoords='offset points',fontsize=9)
  ax.annotate(f'{y1:.1f} dB'.replace('.',','),(1.1,y1),xytext=(-5,-14),ha='right',textcoords='offset points',fontsize=9)
  ax.set_title(f'{names[n]} · vélocité {v}',fontsize=12);ax.set_xticks([0,.3,.5,.7,1.1]);ax.set_xlabel('Temps après attaque (s)');ax.grid(alpha=.16);ax.set_ylim(-50,3)
 axes[0].set_ylabel('Fondamentale / sa propre attaque (dB)')
 handles,labels=axes[0].get_legend_handles_labels();fig.legend(handles,labels,loc='upper center',ncol=3,frameon=False,bbox_to_anchor=(.5,.91),fontsize=10)
 fig.suptitle(title,fontsize=15,y=.99);fig.tight_layout(rect=[0,0,1,.73]);fig.savefig(dest,dpi=160);plt.close(fig);print(dest)
plot([(n,100) for n in selected],'H1 · conserver le gain à 0,5 s, réduire le plateau',R/'docs/img/fundamental_sustain_plateau.png')
plot([(64,v) for v in [45,56,100]],'Mi4 · même comparaison aux trois vélocités',R/'docs/img/fundamental_sustain_plateau_velocities.png')
