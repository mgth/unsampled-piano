from pathlib import Path
import json,sys,subprocess,io
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
p=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent
j=json.load(open(p/'diagnostics.json')); c=json.load(open(p/'candidates.json')); curves=json.load(open(p/'curves.json'))
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False})
fig,axs=plt.subplots(2,2,figsize=(12,8),layout='constrained')
keys=[96,99,102,105,108]; names=['Do7','Ré♯7','Fa♯7','La7','Do8']; ax=axs[0,0]
ref=[]
for m in keys:
 k=f'{m}:8'; ref.append([r[k]['partials'][0] for r in j['references'].values() if k in r and r[k]['partials'][0] is not None])
ax.errorbar(range(5),[np.mean(x) for x in ref],yerr=[np.std(x) for x in ref],fmt='o',color='#777777',capsize=4,label='Réels : moyenne ± écart-type')
for r,label,color in [(j['model']['base'],'Défaut','#ce552f'),(c['soft15'],'Pente 0,15','#e2ad32'),(c['soft10'],'Pente 0,10','#2276aa')]:
 ax.plot([r[f'{m}:8']['partials'][0] for m in keys],'o-',color=color,label=label)
ax.set(xticks=range(5),xticklabels=names,ylabel='P2 / P1 (dB)',title='Deuxième partiel · mezzo-forte · 0–60 ms');ax.legend(fontsize=8);ax.grid(alpha=.15)
t=np.array(curves['times'])+.05
for band,ax in zip([0,1],[axs[0,1],axs[1,0]]):
 refs=np.array(list(curves['references'].values()))[:,:,band]
 ax.fill_between(t,refs.min(0),refs.max(0),color='#cccccc',alpha=.6,label='Étendue des 8 pianos')
 ax.plot(t,refs.mean(0),color='#555555',label='Moyenne des pianos')
 for v,label,color,style in [('base','Défaut','#ce552f','-'),('probe_listen_all','Cordes étouffées à l’écoute','#5f4ba2','--'),('action_long','Bruit mécanique 0,5 s','#41a092','-'),('sympathy2','Sympathie ×2','#bd9440',':')]:
  ax.plot(t,np.array(curves['model'][v])[:,band],color=color,ls=style,label=label)
 ax.set(xlabel='Temps après début détecté (s)',ylabel='Énergie relative (dB)',title=f'Bande {band+1} kHz · forte · fenêtres de 100 ms',ylim=(-85,0));ax.grid(alpha=.15)
 if band==0:ax.legend(fontsize=8)
# Paired-score sensitivities are separate from v2 aggregate z values.
paired=json.load(open(p/'paired-candidates.json')); ax=axs[1,1]
ks=['8:p2','13:p3','13:p4']; labels=['P2 mf\n6 sources','P3 ff\n8 sources','P4 ff\n7 sources']
for offset,v,label,color in [(-.24,'base','Défaut','#ce552f'),(0,'soft10','Pente 0,10','#2276aa'),(.24,'soft10_listen_all','Pente 0,10 + écoute','#5f4ba2')]:
 ax.bar(np.arange(3)+offset,[paired[k]['mean_gap_db'][v] for k in ks],.24,color=color,label=label)
ax.axhline(0,color='#555555',lw=1); ax.set(xticks=range(3),xticklabels=labels,ylabel='Écart modèle − référence (dB)',title='Partiels : touches valides constantes, < 12 kHz');ax.legend(fontsize=8)
fig.suptitle('Suraigu : séparer l’excitation harmonique et le corps sous la fondamentale',fontsize=15)
fig.savefig(p/'top_register.png',dpi=160)
