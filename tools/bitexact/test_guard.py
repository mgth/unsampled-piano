#!/usr/bin/env python3
"""Tests of guard.py's bookkeeping, with a stand-in renderer that writes a
constant: what must never pass for an identical render.

  python3 tools/bitexact/test_guard.py

Linux (the renders go through nice, ionice and taskset, as the real ones).
"""
import contextlib
import io
import os
import shutil
import stat
import struct
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import guard  # noqa: E402

# `… midi IN.mid OUT.wav`: 480 stereo frames of VALUE as 32-bit floats.
RENDERER = """#!/usr/bin/env python3
import os, struct, sys
VALUE = {value}
if os.environ.get("FAKE_RENDER_FAILS"):
    sys.exit("no render today")
data = struct.pack("<f", VALUE) * 960
head = struct.pack("<4sI4s4sIHHIIHH4sI", b"RIFF", 36 + len(data), b"WAVE", b"fmt ", 16, 3, 2, 48000,
                   48000 * 8, 8, 32, b"data", len(data))
with open(sys.argv[-1], "wb") as f:
    f.write(head + data)
print("rendered: ok")
"""
TWO = ["notes_soft", "notes_ff"]


class Guard(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="bitexact_test_")
        self.addCleanup(shutil.rmtree, self.dir)
        self.preset = os.path.join(self.dir, "preset.toml")
        with open(self.preset, "w") as f:
            f.write("# stand-in\n")
        self.a, self.b = self.renderer("a", 0.25), self.renderer("b", 0.5)

    def renderer(self, name, value):
        p = os.path.join(self.dir, f"render_{name}")
        with open(p, "w") as f:
            f.write(RENDERER.format(value=value))
        os.chmod(p, os.stat(p).st_mode | stat.S_IXUSR)
        return p

    def out(self, name):
        return os.path.join(self.dir, name)

    def render(self, binary, out, **kw):
        return guard.render_set(binary, self.preset, self.out(out), jobs=1, quiet=True, only=TWO, **kw)

    def sample(self, out, name):
        with open(os.path.join(self.out(out), name + ".wav"), "rb") as f:
            return struct.unpack("<f", f.read()[44:48])[0]

    def compare(self, a, b):
        """(exit code, {name: identical}) of `guard.py compare`."""
        argv, sys.argv = sys.argv, ["guard.py", "compare", self.out(a), self.out(b)]
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                code = guard.main()
                res = guard.compare_dirs(self.out(a), self.out(b), quiet=True)
        finally:
            sys.argv = argv
        return code, {k: v["identical"] for k, v in res.items()}

    def test_identical_sets_compare_identical(self):
        self.render(self.a, "ref")
        self.render(self.a, "cand")
        self.assertEqual(self.compare("ref", "cand"), (0, {"notes_soft": True, "notes_ff": True}))
        self.render(self.b, "other")
        self.assertEqual(self.compare("ref", "other"), (1, {"notes_soft": False, "notes_ff": False}))

    def test_prune_keeps_a_wav_that_differs_from_its_manifest(self):
        self.render(self.a, "ref")
        self.render(self.a, "cand")
        changed = os.path.join(self.out("cand"), "notes_soft.wav")
        shutil.copyfile(self.other_wav(self.b, "notes_soft"), changed)
        with contextlib.redirect_stdout(io.StringIO()):
            guard.prune(self.out("cand"), self.out("ref"))
        self.assertTrue(os.path.exists(changed), "a render that differs from the reference was deleted")
        man = guard.load_manifest(self.out("cand"))["scenarios"]
        self.assertFalse(man["notes_soft"].get("pruned"))
        self.assertTrue(man["notes_ff"].get("pruned"))
        self.assertFalse(os.path.exists(os.path.join(self.out("cand"), "notes_ff.wav")))
        self.assertEqual(self.compare("ref", "cand"), (1, {"notes_soft": False, "notes_ff": True}))

    def other_wav(self, binary, name):
        """A WAV of the same size as the others, rendered apart by `binary`."""
        self.render(binary, "apart")
        return os.path.join(self.out("apart"), name + ".wav")

    def test_a_failed_render_never_compares_identical(self):
        self.render(self.a, "ref")
        self.render(self.a, "cand")
        os.environ["FAKE_RENDER_FAILS"] = "1"
        try:
            with contextlib.redirect_stderr(io.StringIO()):
                man = self.render(self.a, "cand")
        finally:
            del os.environ["FAKE_RENDER_FAILS"]
        self.assertTrue(all("error" in e for e in man["scenarios"].values()))
        self.assertFalse(os.path.exists(os.path.join(self.out("cand"), "notes_soft.wav")),
                         "the earlier render's WAV outlived the failed one")
        self.assertEqual(self.compare("ref", "cand"), (1, {"notes_soft": False, "notes_ff": False}))
        # Even with a WAV identical to the reference left beside the error.
        for name in TWO:
            shutil.copyfile(os.path.join(self.out("ref"), name + ".wav"), os.path.join(self.out("cand"), name + ".wav"))
        self.assertEqual(self.compare("ref", "cand"), (1, {"notes_soft": False, "notes_ff": False}))
        # And the failed scenarios are rendered again, reuse or not.
        self.render(self.a, "cand", reuse=True)
        self.assertEqual(self.compare("ref", "cand"), (0, {"notes_soft": True, "notes_ff": True}))

    def test_reuse_only_takes_renders_of_the_same_binary(self):
        self.render(self.a, "set")
        with self.assertRaises(SystemExit):  # notes_ff would stay A's in a set said to be B's
            guard.render_set(self.b, self.preset, self.out("set"), jobs=1, quiet=True, only=["notes_soft"], reuse=True)
        self.assertEqual(self.sample("set", "notes_soft"), 0.25)
        self.render(self.b, "set", reuse=True)
        self.assertEqual((self.sample("set", "notes_soft"), self.sample("set", "notes_ff")), (0.5, 0.5))
        man = guard.load_manifest(self.out("set"))
        self.assertTrue(all(e["binary_sha256"] == man["binary_sha256"] for e in man["scenarios"].values()))
        # Same binary again: nothing to do, and the files are left alone.
        before = os.stat(os.path.join(self.out("set"), "notes_ff.wav")).st_mtime_ns
        self.render(self.b, "set", reuse=True)
        self.assertEqual(os.stat(os.path.join(self.out("set"), "notes_ff.wav")).st_mtime_ns, before)
        # A WAV changed behind the manifest's back is rendered again.
        shutil.copyfile(self.other_wav(self.a, "notes_ff"), os.path.join(self.out("set"), "notes_ff.wav"))
        self.render(self.b, "set", reuse=True)
        self.assertEqual(self.sample("set", "notes_ff"), 0.5)


if __name__ == "__main__":
    unittest.main()
