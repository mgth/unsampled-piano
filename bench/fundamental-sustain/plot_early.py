"""Compare the running first H1 trial with the stronger local candidate."""
from pathlib import Path
import json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
R=Path(__file__).resolve().parents[2];O=R/'out/bench/fundamental-sustain'
old=json.loads((O/'prototype.json').read_text())['results']
new=json.loads((O/'early_prototype.json').read_text())['results']
ref=json.loads((R/'out/bench/harmonic_local/harmonic_diagnostic.json').read_text())
times=[.03,.13,.3,.5,.7,.9,1.1]
fig,axes=plt.subplots(1,3,figsize=(12,4.7),sharey=True)
for ax,n,name in zip(axes,[62,64,69],['Ré4','Mi4','La4']):
 k=f'{n}|100';src=ref['notes'][k]
 refs=np.array([[s['decay_self'][t][0] for t in range(7)] for nm,s in src.items() if nm not in ref['models']],float)
 ax.fill_between(times,np.nanmin(refs,axis=0),np.nanmax(refs,axis=0),color='#777777',alpha=.17,label='Min–max des vrais pianos')
 ax.plot(times,np.nanmean(refs,axis=0),color='#555555',linestyle=':',lw=1.5,label='Moyenne des vrais pianos')
 for label,s,color,style,lw in [
  ('Main',src['model'],'#9b7099','--',1.2),
  ('Prototype initial',old['prototype_base'][k],'#bf7626',':',1.5),
  ('Essai précédent · h = 0,1',old['prototype_test'][k],'#2678b7','--',2),
  ('Nouvel essai · h = 0,3',new[k],'#25834b','-',2.3)]:
  ax.plot(times,[s['decay_self'][t][0] for t in range(7)],color=color,ls=style,lw=lw,label=label)
 ax.axvline(.5,color='#888888',lw=.8,alpha=.5)
 ax.scatter([.5],[new[k]['decay_self'][3][0]],color='#25834b',s=24,zorder=5)
 ax.annotate(f"{new[k]['decay_self'][3][0]:.1f} dB".replace('.',','),(.5,new[k]['decay_self'][3][0]),xytext=(5,9),textcoords='offset points',fontsize=10)
 ax.set_title(name,fontsize=13);ax.set_xticks([0,.3,.5,.7,1.1]);ax.set_xlabel('Temps après attaque (s)');ax.grid(alpha=.16);ax.set_ylim(-50,3)
axes[0].set_ylabel('Fondamentale / sa propre attaque (dB)')
handles,labels=axes[0].get_legend_handles_labels();fig.legend(handles,labels,loc='upper center',ncol=3,frameon=False,bbox_to_anchor=(.5,.91),fontsize=10)
fig.suptitle('Tenue de H1 · comparaison du nouvel essai · vélocité 100',fontsize=15,y=.99)
fig.tight_layout(rect=[0,0,1,.78]);dest=R/'docs/img/fundamental_sustain_early.png';fig.savefig(dest,dpi=160);print(dest)
