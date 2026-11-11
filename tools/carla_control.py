#!/usr/bin/env python3
"""Reads and sets the parameters of a plugin hosted in a running Carla, over
Carla's OSC remote control (TCP 22752, on by default). Needs pyliblo.
The plugin is named by a substring of its name in Carla. Docs:
docs/instrument_range.md.
  tools/carla_control.py dump <plugin>              index, name, value of every parameter
  tools/carla_control.py set <plugin> <name> <v>    one parameter, by its exact name
  tools/carla_control.py program <plugin> <name>    loads a program, by its exact name
  tools/carla_control.py dry <plugin>               switches off the delay, equalisers, reverb, limiter and bounce"""
import re, sys, time
import liblo

URL = "osc.tcp://127.0.0.1:22752/Carla"
DRY = re.compile(r"(Eff \d|Reverb|Limiter|Equalizer|Bounce) Switch")


class Carla:
    def __init__(self):
        self.names, self.params, self.values, self.programs = {}, {}, {}, {}
        self.server = liblo.Server(proto=liblo.TCP)
        self.server.add_method(None, None, self.on_message)
        self.target = liblo.Address(URL)
        liblo.send(self.target, "/register", self.server.url)
        self.listen(3.0)
        if not self.names: self.listen(12.0)  # slower on a loaded machine
        if not self.names: sys.exit("no answer from Carla")

    def on_message(self, path, a, *_):
        kind = path.lstrip("/")
        if kind == "info": self.names[a[0]] = a[7]
        elif kind == "paramInfo": self.params.setdefault(a[0], {})[a[1]] = a[2]
        elif kind == "paramData": self.values.setdefault(a[0], {})[a[1]] = a[8]
        elif kind == "param": self.values.setdefault(a[0], {})[a[1]] = a[2]
        elif kind == "prog": self.programs.setdefault(a[0], {})[a[1]] = a[2]

    def listen(self, seconds):
        end = time.time() + seconds
        while time.time() < end: self.server.recv(20)

    def plugin(self, name):
        found = [i for i, n in self.names.items() if name.lower() in n.lower()]
        if len(found) != 1: sys.exit(f"{name!r} matches {len(found)} of {sorted(self.names.values())}")
        return found[0]

    def index(self, plugin, name):
        found = [i for i, n in self.params[plugin].items() if n == name]
        if not found: sys.exit(f"no parameter {name!r}")
        return found[0]

    def set(self, plugin, index, value):
        liblo.send(self.target, f"/Carla/{plugin}/set_parameter_value", ("i", index), ("f", value))

    def program(self, plugin, name):
        found = [i for i, n in self.programs[plugin].items() if n == name]
        if not found: sys.exit(f"no program {name!r} among {sorted(self.programs[plugin].values())}")
        liblo.send(self.target, f"/Carla/{plugin}/set_program", ("i", found[0]))

    def close(self):
        liblo.send(self.target, "/unregister", self.server.url)
        self.server.recv(50)


def show(carla, plugin, indices):
    for i in indices: print(f"{i:4} {carla.params[plugin][i]:36} {carla.values[plugin].get(i)}")


cmd, name = sys.argv[1], sys.argv[2]
carla = Carla()
plugin = carla.plugin(name)
changed = []
if cmd == "dump":
    show(carla, plugin, sorted(carla.params[plugin]))
elif cmd == "set":
    changed = [carla.index(plugin, sys.argv[3])]
    carla.set(plugin, changed[0], float(sys.argv[4]))
elif cmd == "program":
    carla.program(plugin, sys.argv[3])
    carla.listen(4.0)
elif cmd == "dry":
    changed = [i for i, n in sorted(carla.params[plugin].items()) if DRY.fullmatch(n)]
    for i in changed: carla.set(plugin, i, 0.0)
carla.listen(1.5)
carla.close()
if changed:
    # Carla does not report a change to the client that made it: a new client reads the values.
    carla = Carla()
    show(carla, plugin, changed)
    carla.close()
