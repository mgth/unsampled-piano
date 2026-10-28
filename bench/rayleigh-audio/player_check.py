"""Position-player audio check; keep separate from fixed close-mic scores."""
import audio as a
from guard_search import guarded
from pathlib import Path
import numpy as np,json
from scipy.io import wavfile

def main():
 a.IR=np.load(a.O/'kernel-player/responses.npz');a.CACHE.clear();a.GAIN=float(dict(line.split('=') for line in (a.O/'kernel-player/geometry.txt').read_text().splitlines())['mic_gain'])
 preset=a.R/'out/bridge-ear/presets/grid5.toml';folder=a.O/'player-capture';a.c.run([a.BIN,'capture',preset,folder,102,'45,56,100',1.1,.1])
 g=json.load(open(a.P/'guard-selected.json'));out={}
 for v in [45,56,100]:
  dry,new,check=a.stages(folder,102,v);out[v]=dict(reconstruction=check,models={})
  for name,y in [('control',dry),('driven',new),('guarded',guarded(dry,new,v,g['maximum'],g['end'],g['start'],g.get('shape','smooth')))]:
   path=a.O/'player'/name/f'n102v{v}.wav';a.save_audio(preset,y,path);out[v]['models'][name]=a.c.measure_model(path,a.PAR['102'])
 a.c.save(a.P/'player-audio.json',out);print(json.dumps(a.c.clean(out),indent=1))
if __name__=='__main__':main()
