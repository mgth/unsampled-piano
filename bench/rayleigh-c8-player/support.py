"""Controlled player-position continuation; no production/default edits."""
from pathlib import Path
import importlib.util,json,sys,re,hashlib
import numpy as np
from scipy.io import wavfile
from copy import deepcopy
R=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('energy_support',R/'bench/rayleigh-c8-energy/support.py');energy=importlib.util.module_from_spec(spec);spec.loader.exec_module(energy)
prev=energy.prev;a=energy.a;c=energy.c;PAR=energy.PAR
P=R/'bench/rayleigh-c8-player';O=R/'out/rayleigh-c8-player'
CONTACT=json.load(open(energy.P/'selected.json'))['profiles']['recovered']
ENTRIES={'previous':energy.SEED,'recovered':CONTACT['108']}
PLAYER=prev.tomllib.loads((R/'out/bridge-ear/presets/grid5.toml').read_text())['soundboard'];CLOSE=prev.tomllib.loads(prev.BASE)['soundboard']
ACTUAL=prev.tomllib.loads((R/'presets/listening/Microphones/player.toml').read_text())['soundboard']
GEOMETRIES={
 'close':({},None,a.O/'kernel5'),
 'close_dry':({'room.enabled':False},None,a.O/'kernel5'),
 'player35':({'room.enabled':False},PLAYER['mics'],O/'kernel-player35'),
 'player35_room':({},PLAYER['mics'],O/'kernel-player35'),
 'player0':({'room.enabled':False,'soundboard.lid_angle':0},PLAYER['mics'],O/'kernel-player0'),
 'player_actual':({**{f'soundboard.{k}':ACTUAL[k] for k in ['mics_relative','mic_gain_db','mic_pattern','lid_angle']}},ACTUAL['mics'],O/'kernel-player35'),
 'player_actual_dry':({'room.enabled':False,**{f'soundboard.{k}':ACTUAL[k] for k in ['mics_relative','mic_gain_db','mic_pattern','lid_angle']}},ACTUAL['mics'],O/'kernel-player35'),
 'player':({'room.enabled':False,**{f'soundboard.{k}':PLAYER[k] for k in ['mics_relative','mic_gain_db','mic_pattern','grid_step','lid_angle']}},PLAYER['mics'],a.O/'kernel-player'),
}
def preset(e,geom,path,extra=None):
 fields,mics,_=GEOMETRIES[geom];ee=deepcopy(e);ee['extra'].update(fields);ee['extra'].update(extra or {});prev.preset(ee,path)
 if mics is not None:path.write_text(re.sub(r'(?ms)^mics = \[.*?^\]$','mics = '+json.dumps(mics),path.read_text()))
 return path

def kernel(geom):
 folder=GEOMETRIES[geom][2];a.IR=np.load(folder/'responses.npz');a.CACHE.clear();a.GAIN=float(dict(line.split('=') for line in (folder/'geometry.txt').read_text().splitlines())['mic_gain'])
 if geom.startswith('player_actual'):a.GAIN=10**(ACTUAL['mic_gain_db']/20)

def capture(e,name,geom,vel=(45,56,100),hold=.2,tail=.05,extra=None,root=None,keep=True,note=108):
 d=(O/'capture' if root is None else root)/geom/name;p=preset(e,geom,d/'preset.toml',extra);cap=d/'stages';c.run([a.BIN,'capture',p,cap,note,','.join(map(str,vel)),hold,tail]);m={};checks={}
 for v in vel:
  dry,new,check=a.stages(cap,note,v);assert check['rms_error_db'] < -90,(geom,name,check);path=d/f'n{note}v{v}.wav';a.save_audio(p,new,path);m[str(v)]=prev.measure(path,note);checks[str(v)]=check
  if not keep:(cap/f'n{note}v{v}.force.f32').unlink()
 c.save(d/'metrics.json',m);c.save(d/'checks.json',checks);return m
