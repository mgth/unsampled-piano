"""Common-gain listening montage and scientific summary, no metric edits."""
from common import *
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import importlib.util
spec=importlib.util.spec_from_file_location('contact_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)

def main():
 names=['balanced','driven','contact'];labels=['Témoin précédent','Intégral, ancien contact','Intégral, contact ajusté'];colors=['#6d7885','#c26c30','#28779b'];dense=json.load(open(P/'validation-dense.json'));summary=json.load(open(P/'validation-summary.json'));fig,axes=plt.subplots(2,2,figsize=(12,8),layout='constrained')
 for ax,n,k,title in [(axes[0,0],102,3,'Fa♯7 : P3 / P1'),(axes[0,1],108,2,'Do8 : P2 / P1')]:
  for name,label,color in zip(names,labels,colors):ax.plot(range(28,128),[dense[name][f'{n}:{v}']['partials'][k-2] for v in range(28,128)],label=label,color=color)
  if n==102:ax.plot(range(28,128),[dense['contact_pp'][f'{n}:{v}']['partials'][k-2] for v in range(28,128)],'--',color='#28779b',label='Variante pp')
  rows=summary['attack_rows'];ax.scatter([45,56,100],[np.mean([r['target'] for r in rows if r['note']==n and r['velocity']==v and r['k']==k]) for v in [45,56,100]],marker='x',color='black',label='Moyenne des références disponibles');ax.set(title=title,xlabel='Vélocité MIDI',ylabel='dB relatifs à P1');ax.grid(alpha=.2)
 axes[0,0].legend(fontsize=8)
 for i,(key,label) in enumerate([('fsharp','Fa♯7'),('c8','Do8'),('attack','C7–Do8'),('sustain','Tenue')]):
  for j,(name,color) in enumerate(zip(names,colors)):axes[1,0].bar(i+(j-1)*.24,summary[key][name]['mae'],width=.22,color=color)
 axes[1,0].set(xticks=range(4),xticklabels=['Fa♯7','Do8','C7–Do8','Tenue'],ylabel='MAE, dB',title='Même couverture, même poids par piano');axes[1,0].grid(axis='y',alpha=.2)
 for name,label,color in zip(names[1:],labels[1:],colors[1:]):
  axes[1,1].plot([45,56,100],[summary['band_changes'][name]['108'][str(v)]['p1'] for v in [45,56,100]],'o-',label=f'{label} : P1',color=color)
  axes[1,1].plot([45,56,100],[summary['band_changes'][name]['108'][str(v)]['bands']['1-2k'] for v in [45,56,100]],'s--',label=f'{label} : 1–2 kHz',color=color,alpha=.7)
 axes[1,1].axhline(0,color='gray',lw=.7);axes[1,1].set(xlabel='Vélocité MIDI',ylabel='Variation absolue, dB',title='Do8 : énergie par rapport au témoin');axes[1,1].legend(fontsize=8);axes[1,1].grid(alpha=.2)
 fig.savefig(R/'docs/img/rayleigh_contact.png',dpi=160);plt.close(fig)
 chunks=[];timeline=[];cursor=0
 for n,v in [(102,45),(102,100),(108,45),(108,56),(108,100)]:
  for name,label in zip(names,labels):
   sr,x=wavfile.read(val.file('attack',name,n,v));assert sr==48000;x=x.astype(float);x[-960:]*=np.linspace(1,0,960)[:,None];timeline.append(dict(note=n,velocity=v,variant=name,start_seconds=cursor/48000));chunks.extend([x,np.zeros((14400,2))]);cursor+=len(x)+14400
 x=np.concatenate(chunks);gain=min(1,.85/np.max(abs(x)));wavfile.write(O/'comparison.wav',48000,(x*gain).astype(np.float32));c.save(P/'listening.json',dict(gain=gain,timeline=timeline,policy='One common gain for every item; identical 20 ms end fade only in montage, 300 ms silence. Measurements use untouched WAVs.'))
if __name__=='__main__':main()
