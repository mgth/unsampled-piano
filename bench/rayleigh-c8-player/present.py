"""Show the spatial bottleneck and candidate costs; one-gain player montage."""
from support import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
spec=importlib.util.spec_from_file_location('player_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)

def main():
 diag=json.load(open(P/'diagnostic.json'));s=json.load(open(P/'validation-summary.json'));dense=json.load(open(P/'validation-dense.json'));names=val.NAMES;labels=['Contact conservé','Feutre plus souple','Masse +5 %','Masse +10 %, hampe 350 Hz'];colors=['#287ca0','#649247','#ad7930','#b64e4c'];fig,ax=plt.subplots(2,2,figsize=(12,8),layout='constrained')
 for i,(geom,label) in enumerate([('close_dry','Micros rapprochés'),('player35','Position pianiste')]):
  d=diag['components'][geom]['recovered']['56']
  for j,(part,color) in enumerate([('board','#287ca0'),('action','#ad7930')]):ax[0,0].bar(i+(j-.5)*.3,d[part]['trigger60']+100,bottom=-100,width=.28,color=color,label=('Table' if part=='board' else 'Action') if i==0 else None)
 ax[0,0].set(xticks=[0,1],xticklabels=['Micros rapprochés','Position pianiste'],ylabel='Niveau 1–2 kHz, dBFS',title='Do8 mf : position seule, sans salle');ax[0,0].legend();ax[0,0].set_ylim(-95,-60)
 for geom,label,color in [('close','Micros rapprochés','#287ca0'),('player','Ancienne sonde : sans couvercle/salle','#b64e4c'),('player_actual','Preset pianiste 35° + studio','#649247')]:
  m=diag['models'][geom];ax[0,1].plot([45,56,100],[m['recovered'][str(v)]['bands']['1-2k']-m['previous'][str(v)]['bands']['1-2k'] for v in [45,56,100]],'o-',label=label,color=color)
 ax[0,1].set(xlabel='Vélocité MIDI',ylabel='Variation 1–2 kHz, dB',title='Effet de la correction du contact précédente');ax[0,1].legend(fontsize=8);ax[0,1].axhline(0,lw=.6,color='gray')
 for name,label,color in zip(names,labels,colors):
  ax[1,0].plot(range(28,128),[dense['close'][name][f'108:{v}']['partials'][0] for v in range(28,128)],label=label,color=color)
  ax[1,1].plot([45,56,100],[s['stats']['player_actual'][name]['delta'][str(v)]['1-2k'] for v in [45,56,100]],'o-',label=label,color=color)
 ax[1,0].set(title='Creux P2/P1 : banc rapproché',xlabel='Vélocité MIDI',ylabel='P2 / P1, dB');ax[1,0].legend(fontsize=8)
 ax[1,1].set(title='Gain de bande basse au pianiste, nouvelles pistes',xlabel='Vélocité MIDI',ylabel='dB par rapport au contact conservé');ax[1,1].legend(fontsize=8)
 for aa in ax.flat:aa.grid(alpha=.18)
 fig.savefig(R/'docs/img/rayleigh_c8_player.png',dpi=160);plt.close(fig)
 chunks=[];timeline=[];cursor=0
 for v in [45,56,100]:
  for name in ['baseline','soft','energy']:
   sr,x=wavfile.read(val.file('attack','player_actual',name,108,v));assert sr==48000;x=x.astype(float);x[-960:]*=np.linspace(1,0,960)[:,None];timeline.append(dict(velocity=v,variant=name,start_seconds=cursor/48000));chunks.extend([x,np.zeros((14400,2))]);cursor+=len(x)+14400
 x=np.concatenate(chunks);gain=.85/np.max(abs(x));wavfile.write(O/'comparison.wav',48000,(gain*x).astype(np.float32));c.save(P/'listening.json',dict(common_gain=gain,timeline=timeline,policy='Actual player preset + studio. One common gain, identical 20 ms end fade in montage only. Comparison candidates are not promoted.'))
if __name__=='__main__':main()
