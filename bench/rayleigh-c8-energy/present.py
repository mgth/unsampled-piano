"""Fixed-gain listening and diagnostic plots; benchmark WAVs remain untouched."""
from support import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
spec=importlib.util.spec_from_file_location('c8_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)

def main():
 summary=json.load(open(P/'validation-summary.json'));curves=json.load(open(P/'energy-guards.json'))['dense_curves'];names=['balanced','contact','recovered','recovered_mf'];labels=['Témoin historique','Contact précédent','Volume récupéré','Variante mf'];colors=['#7c8794','#c37631','#237ca0','#578752'];fig,ax=plt.subplots(2,2,figsize=(12,8),layout='constrained');vel=list(range(28,128))
 for name,label,color in zip(names,labels,colors):
  style='--' if name=='recovered_mf' else '-';ax[0,0].plot(vel,[curves[name][str(v)]['partials'][0] for v in vel],style,label=label,color=color)
  for a1,key in [(ax[0,1],'level_preroll'),(ax[1,0],'1-2k')]:
   y=[curves[name][str(v)]['bands'][key]-curves['balanced'][str(v)]['bands'][key] if key=='1-2k' else curves[name][str(v)][key]-curves['balanced'][str(v)][key] for v in vel];a1.plot(vel,y,style,color=color,label=label)
 ax[0,0].scatter([45,56,100],[np.mean([r['target'] for r in summary['attack_rows'] if r['note']==108 and r['velocity']==v]) for v in [45,56,100]],marker='x',color='black');ax[0,0].set(title='Do8 : P2 / P1',ylabel='dB relatifs à P1');ax[0,0].legend(fontsize=8)
 ax[0,1].set(title='Volume récupéré, toutes les vélocités',ylabel='dB par rapport au témoin historique');ax[1,0].set(title='Bande 1–2 kHz',ylabel='dB par rapport au témoin historique')
 for a1 in [ax[0,0],ax[0,1],ax[1,0]]:a1.set_xlabel('Vélocité MIDI');a1.grid(alpha=.2)
 for i,v in enumerate([45,56,100]):
  for j,(name,color) in enumerate(zip(names,colors)):ax[1,1].bar(i+(j-1.5)*.18,summary['c8_dynamics'][str(v)][name]['mae'],width=.17,color=color)
 ax[1,1].set(xticks=range(3),xticklabels=['pp (45)','mf (56)','forte (100)'],title='Erreur P2 à couverture constante',ylabel='MAE pondérée par piano, dB');ax[1,1].grid(axis='y',alpha=.2);fig.savefig(R/'docs/img/rayleigh_c8_energy.png',dpi=160);plt.close(fig)
 chunks=[];timeline=[];cursor=0
 for v in [45,56,100]:
  for name in ['balanced','contact','recovered']:
   sr,x=wavfile.read(val.file('attack',name,108,v));assert sr==48000;x=x.astype(float);x[-960:]*=np.linspace(1,0,960)[:,None];timeline.append(dict(velocity=v,variant=name,start_seconds=cursor/48000));chunks.extend([x,np.zeros((14400,2))]);cursor+=len(x)+14400
 x=np.concatenate(chunks);gain=.85/np.max(abs(x));wavfile.write(O/'comparison.wav',48000,(gain*x).astype(np.float32));c.save(P/'listening.json',dict(common_gain=gain,timeline=timeline,policy='One common gain for all items. Identical 20 ms end fade in montage only. No per-note normalization.'))
if __name__=='__main__':main()
