from support import *

def main():
 spec=importlib.util.spec_from_file_location('rayleigh_filters',R/'bench/rayleigh-audio/filters.py');f=importlib.util.module_from_spec(spec);spec.loader.exec_module(f);f.P=P
 for geom in ['player35','player0']:
  p=preset(ENTRIES['recovered'],geom,O/'presets'/f'{geom}.toml');folder=GEOMETRIES[geom][2];c.run([a.O/'bin/rayleigh_kernel',p,folder,.005,32]);f.make(folder)
if __name__=='__main__':main()
