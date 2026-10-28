"""Position/lid/room controls and coherent acoustic-path decomposition."""
from support import *

def power(x,start=0,size=2880):
 seg=x[start:start+size];w=np.hanning(len(seg));X=np.fft.rfft(seg*w[:,None],n=32768,axis=0);freq=np.fft.rfftfreq(32768,1/48000);ps=2*abs(X)**2/(48000*np.sum(w*w))*48000/32768;ix=(freq>=1000)&(freq<2000);return float(np.mean(np.sum(ps[ix],axis=0)))
def metrics(x):return dict(own60=prev.bands(x)['1-2k'],trigger60=10*np.log10(power(x)+1e-30),fixed80_60=10*np.log10(power(x,80)+1e-30),trigger200=10*np.log10(power(x,0,9600)+1e-30))

def main():
 out={};force_hashes={};components={}
 for geom in GEOMETRIES:
  kernel(geom);out[geom]={};components[geom]={}
  for name,e in ENTRIES.items():
   path=O/'capture'/geom/name/'metrics.json';out[geom][name]=json.load(open(path)) if path.exists() else capture(e,name,geom);folder=O/'capture'/geom/name;cap=folder/'stages';components[geom][name]={}
   if geom in ['close','close_dry','player35','player','player_actual','player_actual_dry']:

    if not (O/'capture'/geom/(name+'_noaction')/'metrics.json').exists():capture(e,name+'_noaction',geom,extra={'noise.action':0})
   for v in [45,56,100]:
    h=hashlib.sha256((cap/f'n108v{v}.force.f32').read_bytes()).hexdigest();key=f'{name}:{v}'
    if key in force_hashes:assert h==force_hashes[key],(geom,key)
    force_hashes[key]=h
    dry,total,check=a.stages(cap,108,v);_,four=wavfile.read(cap/f'n108v{v}.wav');res=dry-four[:,2:];board=total-res;signals={'total':total,'board':board,'residual':res}
    if geom in ['close','close_dry','player35','player','player_actual','player_actual_dry']:
     cap0=O/'capture'/geom/(name+'_noaction')/'stages';assert (cap0/f'n108v{v}.force.f32').read_bytes()==(cap/f'n108v{v}.force.f32').read_bytes();dry0,_,_=a.stages(cap0,108,v);signals['action']=dry-dry0;signals['other']=res-signals['action']
    post={}
    for label,y in signals.items():
     path=O/'decomposition'/geom/name/f'{label}-v{v}.wav';a.save_audio(folder/'preset.toml',y,path);_,post[label]=wavfile.read(path)
    values={label:metrics(y) for label,y in post.items()};values['coherent_cross_60']=power(post['total'])-power(post['board'])-power(post['residual']);components[geom][name][str(v)]=values
 c.save(P/'diagnostic.json',dict(models=out,components=components,force_hashes=force_hashes,policy='Identical bridge forces across position/lid/room controls, checked byte-for-byte. No-action capture also preserves forces. Component powers are coherent: do not add dB or infer percentages without cross term. Fixed 60/200 ms windows accompany the original onset-based metric.'))
 print(json.dumps(c.clean({g:{str(v):out[g]['recovered'][str(v)]['bands']['1-2k']-out[g]['previous'][str(v)]['bands']['1-2k'] for v in [45,56,100]} for g in out}),indent=1))
if __name__=='__main__':main()
