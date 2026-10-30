import json
from pathlib import Path
HERE = Path(__file__).resolve().parent
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
r=json.load(open(HERE / 'audit.json'))
fig,axes=plt.subplots(1,2,figsize=(13,5.6),layout='constrained')
fig.suptitle('Deux raisons de ne pas optimiser le score global seul',fontsize=16,fontweight='bold')
a=axes[0]; rows=r['scores']['real_leave_one_out']; names=list(rows);y=np.arange(len(names))
a.scatter([rows[n]['common']['score'] for n in names],y,color='#136f63',label='Vrai piano retiré des cibles',s=48)
a.scatter([rows[n]['model_same_targets_common']['score'] for n in names],y,color='#d46a27',marker='x',label='Modèle contre les mêmes cibles',s=50)
a.set_yticks(y,names);a.set_ylim(9.1,-.5);a.set_xlabel('RMS z, mêmes 112 mesures (plus bas = plus proche)');a.set_title('Chaque piano réel comparé aux sept autres');a.grid(axis='x',alpha=.2);a.legend(fontsize=9,loc='lower right');a.set_xlim(0,2)
b=axes[1];candidates=[]
for source in ['default','close_live_touch','close_playback','bivib','salamander']:
 candidates.append(next(x for x in r['probes']+r['reference_probes'] if x['source']==source and x['midi']==84 and (x.get('velocity')==56 or x.get('layer')==8)))
labels=['Modèle : défaut, v56','Modèle : proche, v56','Modèle : banc, v56','BiVib : v56','Salamander : couche 8']
for i,x in enumerate(candidates):
 b.plot([x['attack']['p2_over_p1_stereo_db'],x['attack']['p2_over_p1_mono_db']],[i,i],color='#b8b8b8',lw=2)
b.scatter([x['attack']['p2_over_p1_stereo_db'] for x in candidates],range(5),label='Puissance moyenne L/R',color='#136f63',s=48)
b.scatter([x['attack']['p2_over_p1_mono_db'] for x in candidates],range(5),label='Somme mono',color='#8057a7',marker='x',s=50)
b.set_yticks(range(5),labels);b.set_xlabel('Partiel 2 / partiel 1 (dB), attaque 0–60 ms');b.set_title('C6 : piano, écoute et méthode changent la mesure');b.grid(axis='x',alpha=.2);b.legend(fontsize=9,loc='lower right');b.set_xlim(-43,-5);b.set_ylim(5.8,-.6)
fig.supxlabel('Mesures diagnostiques : prises de son et échelles de vélocité distinctes ; aucun classement de réalisme.',fontsize=10)
fig.savefig(HERE.parent.parent / 'docs/img/realism_audit.png',dpi=170)
