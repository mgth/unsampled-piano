from pathlib import Path
import json,sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
p=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent
models=json.load(open(p/'validation-models.json'));refs=json.load(open(p/'references.json'));summary=json.load(open(p/'summary.json'));odd=json.load(open(p/'odd-check.json'))
colors={'base':'#c45632','q_only':'#c7a243','shaped':'#8875b3','guarded':'#207a9d'}
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False})
fig,axs=plt.subplots(2,2,figsize=(12,8),layout='constrained')
ax=axs[0,0]
for n,label in [('base','Défaut'),('guarded','Profil retenu')]:
 r=dict(models[n]);r.update(odd[n]);vs=sorted(int(k.split(':')[1]) for k in r if k.startswith('108:'))
 ax.plot(vs,[r[f'108:{v}']['partials'][0] for v in vs],label=label,color=colors[n])
for i,v in enumerate([45,56,100]):
 vals=[r[f'108:{v}']['partials'][0] for r in refs.values() if f'108:{v}' in r and r[f'108:{v}']['partials'][0] is not None]
 ax.errorbar(v,np.mean(vals),yerr=np.std(vals),fmt='o',color='#555555',capsize=4,label='Pianos réels ± écart-type' if i==0 else None)
ax.set(title='Do8 : P2, toutes les vélocités de 28 à 127',xlabel='Vélocité MIDI',ylabel='P2 / P1 (dB)');ax.legend(fontsize=8);ax.grid(alpha=.15)
ax=axs[0,1]
r=dict(models['guarded']);r.update(odd['guarded']);base=dict(models['base']);base.update(odd['base'])
vs=sorted(int(k.split(':')[1]) for k in r if k.startswith('108:'))
ax.plot(vs,[r[f'108:{v}']['level']-base[f'108:{v}']['level'] for v in vs],color=colors['guarded']);ax.axhline(0,color='#777',lw=1)
ax.set(title='Do8 : niveau de l’attaque conservé',xlabel='Vélocité MIDI',ylabel='Profil − défaut (dB)');ax.grid(alpha=.15)
ax=axs[1,0];keys=['held_sources_all','held_notes_all','pp','all_top'];labels=['2 pianos\nhors réglage','Touches\nintermédiaires','Pianissimo¹','Tout le\nsuraigu']
for shift,n,label in [(-.24,'base','Défaut'),(0,'shaped','Profil initial'),(.24,'guarded','Profil retenu')]:
 ax.bar(np.arange(4)+shift,[summary['scores'][k][n]['mae'] for k in keys],.24,label=label,color=colors[n])
ax.set(xticks=range(4),xticklabels=labels,ylabel='Écart absolu moyen (dB)',title='Même couverture de touches, nuances et partiels');ax.legend(fontsize=8)
ax=axs[1,1];keys=list(range(96,109))
for n,label in [('base','Défaut'),('guarded','Profil retenu')]:ax.plot(keys,[models[n][f'{m}:56']['partials'][0] for m in keys],'o-',label=label,color=colors[n])
for i,m in enumerate(keys):
 vals=[r[f'{m}:56']['partials'][0] for r in refs.values() if f'{m}:56' in r and r[f'{m}:56']['partials'][0] is not None]
 if vals:ax.errorbar(m,np.mean(vals),yerr=np.std(vals),fmt='.',color='#777777',capsize=2,label='Références disponibles' if i==0 else None)
ax.set(title='P2 mezzo-forte : toutes les touches du suraigu',xlabel='Note MIDI (96 = Do7, 108 = Do8)',ylabel='P2 / P1 (dB)');ax.legend(fontsize=8);ax.grid(alpha=.15)
fig.suptitle('Contact par touche : Do8 préservé, amélioration encore inégale',fontsize=15)
fig.supxlabel('¹ Le pianissimo a servi à corriger le second profil : ce n’est plus une validation indépendante.',fontsize=9)
fig.savefig(p/'contact_voicing.png',dpi=160)
