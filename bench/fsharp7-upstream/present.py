from study import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

S=json.load(open(P/'validation-summary.json'));M=json.load(open(P/'validation-models.json'));D=json.load(open(P/'validation-dense.json'))
labels={'control':'Contact actuel','freq375_h0.2':'Hampe 375 Hz, h=0,2','strike0.108_h0.15':'Frappe 0,108, h=0,15','hysteresis_0.15':'Hystérésis 0,15','s0.098_h0.2_m1.05':'Frappe 0,098, masse +5 %, h=0,2'}
colors=['#555555','#1976b9','#e27e20','#309b70','#a54d9e']
time={};p=PAR['102'];k=np.arange(1,5);f=p['f0']*k*np.sqrt(1+p['B']*k*k)
for name in labels:
 time[name]={}
 for v in VEL:
  x=b.load(str(O/name/'attack'/f'n102v{v}.wav'),stereo=True)
  time[name][v]={f'{a}:{dur}':c.attack(x,f,a,dur) for a,dur in [(0,.06),(.06,.06),(.12,.06),(.25,.1),(.5,.5)]}
save(P/'temporal.json',time)
fig,axes=plt.subplots(2,2,figsize=(13,8),layout='constrained')
for j,(name,label) in enumerate(labels.items()):
 color=colors[j]
 for i in [0,1]:axes[0,i].plot(range(28,128),[D[name][f'102:{v}']['partials'][i] for v in range(28,128)],label=label,color=color,lw=1.8)
 for i in [0,1]:
  if name=='control':continue
  if i==0:y=[time[name][v]['0.5:0.5']['partials'][0]-time['control'][v]['0.5:0.5']['partials'][0] for v in VEL]
  else:y=[M[name][f'102:{v}']['level_preroll']-M['control'][f'102:{v}']['level_preroll'] for v in VEL]
  axes[1,i].plot(['pp (45)','mf (56)','f (100)'],y,'o-',color=color,label=label)
for i,k in enumerate([2,3]):
 axes[0,i].set(title=f'Fa♯7 — attaque P{k}/P1',xlabel='Vélocité MIDI',ylabel='dB / P1');axes[0,i].grid(alpha=.2)
axes[1,0].set(title='P2 tenu (0,5–1 s) : variation face au contact actuel',ylabel='dB ; positif = écart accru aux références')
axes[1,1].set(title='Niveau d’attaque : variation face au contact actuel',ylabel='dB, sans normalisation')
for ax in axes[1]:ax.axhline(0,color='#555555',lw=1);ax.grid(alpha=.2)
axes[0,0].legend(fontsize=8)
fig.suptitle('Fa♯7 après upstream — contacts locaux, unissons et cadre fixes\nÉcoute rapprochée du banc ; aucun candidat retenu',fontsize=14)
out=R/'docs/img/fsharp7_upstream.png';out.parent.mkdir(parents=True,exist_ok=True);fig.savefig(out,dpi=160);plt.close(fig)
rows=[]
for name,label in labels.items():
 s=S['targets'][name];dip=S['dips'][name]['102'];t=time[name]
 rows.append(dict(name=name,label=label,ff_p3_bias=s['ff_p3_bias'],ff_p3_absolute_delta=s['delta']['100']['p1']+s['delta']['100']['partials'][1],attack_mae={str(v):S['scores'][f'fsharp_v{v}'][name]['mae'] for v in VEL},held_p2_delta={str(v):t[v]['0.5:0.5']['partials'][0]-time['control'][v]['0.5:0.5']['partials'][0] for v in VEL},level_delta={str(v):s['delta'][str(v)]['level'] for v in VEL},dips=dip))
save(P/'comparison.json',rows)
print(json.dumps(c.clean(rows),indent=1))
