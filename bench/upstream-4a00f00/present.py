from common import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
x=json.load(open(P/'summary.json'));d=json.load(open(P/'dense.json'))['close'];f=json.load(open(P/'full-summary.json'));ab=json.load(open(P/'ablation.json'))['metrics']
fig,axes=plt.subplots(2,2,figsize=(11,7),layout='constrained')
a=axes[0,0];values=[f['previous']['score'],f['profiles']['default']['fixed_summary']['score'],f['profiles']['integral']['fixed_summary']['score']];bars=a.bar(['Défaut avant','Défaut upstream','Test intégral'],values,color=['#667085','#3382b8','#e8a13a']);a.bar_label(bars,fmt='%.3f');a.set_ylim(0,1.5);a.set_title('Banc long : 191 composantes identiques');a.set_ylabel('RMS z (plus bas = mieux)')
for a,n,k,title in [(axes[0,1],102,1,'Fa♯7 : P3/P1'),(axes[1,0],108,0,'Do8 : P2/P1')]:
 for name,label,color in [('previous_integral','Avant upstream','#667085'),('integral','Après upstream','#3382b8')]:
  a.plot(range(28,128),[d[name][f'{n}:{v}']['partials'][k] for v in range(28,128)],label=label,color=color)
 a.set_title(title+' — contacts inchangés');a.set_xlabel('Vélocité MIDI');a.set_ylabel('dB');a.grid(alpha=.2);a.legend(fontsize=9)
a=axes[1,1];names=['width2','frame_length175','frame_oneway','frame_loss1'];labels=['Courbe t²','Cadre 1,75 m','Cadre sens unique','Pertes cadre 1 %'];base=ab['current']['102:100']['partials'][1];bars=a.barh(labels,[ab[n]['102:100']['partials'][1]-base for n in names],color='#3382b8');a.bar_label(bars,fmt='%+.2f',padding=3);a.set_xlim(-.35,1.7);a.axvline(0,color='gray',linewidth=.7);a.set_title('Fa♯7 forte : ablations diagnostiques');a.set_xlabel('Variation de P3/P1 face à upstream, dB')
fig.suptitle('Intégration upstream 4a00f00 — aucun nouveau réglage retenu',fontsize=14)
fig.savefig(R/'docs/img/upstream_4a00f00.png',dpi=150);plt.close(fig)
