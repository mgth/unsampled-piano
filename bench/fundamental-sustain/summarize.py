"""Summarise the fixed note/velocity validation, without promoting a preset."""
from pathlib import Path
import json,hashlib
import numpy as np
from scipy.io import wavfile
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
R=Path(__file__).resolve().parents[2];O=R/'out/bench/fundamental-sustain';P=Path(__file__).resolve().parent
x=json.loads((O/'prototype.json').read_text());ref=json.loads((R/'out/bench/harmonic_local/harmonic_diagnostic.json').read_text())
maxattack=[];maxupper=[];peaks=[];releases=[];rows=[]
for n in x['changed_keys']:
 for v in x['velocities']:
  key=f'{n}|{v}';base=x['results']['prototype_base'][key];test=x['results']['prototype_test'][key]
  a=np.array(base['levels'],float);z=np.array(test['levels'],float)
  maxattack.append(abs(z[0,0]-a[0,0]));maxupper.extend(abs(z[7,1:]-a[7,1:]))
  sr,w=wavfile.read(O/'prototype_test'/f'n{n}v{v}.wav');assert np.isfinite(w).all();peaks.append(float(abs(w).max()))
  before=np.sqrt(np.mean(w[int(3.8*sr):int(4*sr)]**2));after=np.sqrt(np.mean(w[int(4.3*sr):int(4.5*sr)]**2))
  releases.append(float(20*np.log10(after/before)))
  sources={nm:s['decay_self'][6][0] for nm,s in ref['notes'][key].items() if nm not in ref['models'] and s['decay_self'][6][0] is not None}
  rows.append(dict(note=n,velocity=v,main=ref['notes'][key]['model']['decay_self'][6][0],prototype=base['decay_self'][6][0],candidate=test['decay_self'][6][0],delta_absolute=z[6,0]-a[6,0],references=sources,reference_min=min(sources.values()),reference_max=max(sources.values())))
controls={k:v for k,v in x['checks'].items() if int(k.split('|')[0]) not in x['changed_keys']}
assert len(controls)==27 and all(c['identical'] for c in controls.values())
sr,a=wavfile.read(O/'base/n63v100.wav');_,b=wavfile.read(R/'out/bench/harmonic_local/notes/D#4v13.wav');assert np.array_equal(a[:int(1.4*sr)],b[:int(1.4*sr)])
checks=dict(unchanged_control_renders=len(controls),controls=controls,max_h1_attack_change_db=max(maxattack),max_h2_h8_sustain_level_change_db=float(np.nanmax(maxupper)),max_audio_peak=max(peaks),release_drop_db_range=[min(releases),max(releases)],probe_matches_main_first_1_4_seconds=True)
result=dict(changed_keys=x['changed_keys'],fields=x['fields'],method='H1 peak at 1.0–1.2 s relative to same H1 at 0–60 ms; mean stereo FFT power, RMS onset',rows=rows,checks=checks)
(P/'results.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in checks.items() if k!='controls'},indent=2))
fig,axes=plt.subplots(3,3,figsize=(12,8.5),sharex=True,sharey=True)
times=np.array([.03,.13,.3,.5,.7,.9,1.1]); names={62:'Ré4',63:'Ré♯4',64:'Mi4'}
for i,n in enumerate([62,63,64]):
 for j,v in enumerate([45,56,100]):
  key=f'{n}|{v}';ax=axes[i,j];sources=ref['notes'][key]
  rs=np.array([[s['decay_self'][t][0] for t in range(7)] for nm,s in sources.items() if nm not in ref['models']],float)
  ax.fill_between(times,np.nanmin(rs,axis=0),np.nanmax(rs,axis=0),color='#777777',alpha=.17,label='Étendue des vrais pianos')
  ax.plot(times,np.nanmean(rs,axis=0),color='#555555',linestyle=':',lw=1.3,label='Moyenne des vrais pianos')
  series=[('Main',sources['model'],'#2872aa','--'),('Prototype',x['results']['prototype_base'][key],'#bd651b','--'),('Essai local',x['results']['prototype_test'][key],'#21874a','-')]
  for label,s,c,ls in series:ax.plot(times,[s['decay_self'][t][0] for t in range(7)],color=c,lw=1.8,ls=ls,label=label)
  ax.set_title(f'{names[n]} · vélocité {v}',fontsize=11);ax.grid(alpha=.18);ax.set_xticks([0,.3,.7,1.1]);ax.set_ylim(-60,4)
  if j==0:ax.set_ylabel('H1 / H1 à l’attaque (dB)')
  if i==2:ax.set_xlabel('Temps après attaque (s)')
handles,labels=axes[0,0].get_legend_handles_labels();fig.legend(handles,labels,loc='upper center',ncol=3,frameon=False,bbox_to_anchor=(.5,.965))
fig.suptitle('Retrouver la tenue de la fondamentale · essai de couplage horizontal par touche',fontsize=14,y=.995)
fig.tight_layout(rect=[0,0,1,.90]);dest=R/'docs/img/fundamental_sustain.png';fig.savefig(dest,dpi=150);plt.close(fig)
print(dest)
