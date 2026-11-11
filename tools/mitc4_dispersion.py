#!/usr/bin/env python3
"""Dispersion of the built board's elements (crates/piano-core/src/fem.rs, docs/real_board.md,
"Above 3 kHz"): a plane wave on an endless regular grid of MITC4 elements over a uniform spruce
plate, against the Reissner-Mindlin plate's own. Error in frequency at the true frequency, mean
over the directions and the worst one, per grid step and per mass matrix: consistent, lumped,
and (1 + over) x lumped - over x consistent.
  mitc4_dispersion.py [thickness, m] [over]"""
import sys
import numpy as np

el, er, g, nu, gl, gr, rho = 11.6e9, 0.9e9, 0.75e9, 0.37, 0.72e9, 0.039e9, 390.0
def rotated(c, s):
    k = 1/(1-nu*nu*er/el); q11, q22, q12, q66 = k*el, k*er, k*nu*er, g
    c2, s2 = c*c, s*s; c4, s4, sc2 = c2*c2, s2*s2, s2*c2
    return np.array([[q11*c4+2*(q12+2*q66)*sc2+q22*s4, (q11+q22-4*q66)*sc2+q12*(s4+c4), (q11-q12-2*q66)*s*c*c2+(q12-q22+2*q66)*s*s2*c],
                     [0, q11*s4+2*(q12+2*q66)*sc2+q22*c4, (q11-q12-2*q66)*s*s2*c+(q12-q22+2*q66)*s*c*c2],
                     [0, 0, (q11+q22-2*q12-2*q66)*sc2+q66*(s4+c4)]])
def material(h, ang):
    c, s = np.cos(ang), np.sin(ang); Q = rotated(c, s); Q = Q + np.triu(Q, 1).T
    D = Q*h**3/12; S = 5/6*h*np.array([[c*c*gl+s*s*gr, c*s*(gl-gr)], [c*s*(gl-gr), s*s*gl+c*c*gr]])
    return D, S, rho*h, rho*h**3/12
GA = [(0.06943184420297371, 0.17392742256872693), (0.33000947820757187, 0.32607257743127307), (0.6699905217924281, 0.32607257743127307), (0.9305681557970263, 0.17392742256872693)]
def element(hx, hy, D, S, m, I, reduced=False):
    ke = np.zeros((12, 12)); me = np.zeros((12, 12)); lump = np.zeros(12)
    for eta, w2 in GA:
        for xi, w1 in GA:
            N = [(1-xi)*(1-eta), xi*(1-eta), (1-xi)*eta, xi*eta]
            nx = [1-xi, xi, 1-xi, xi]; ny = [1-eta, 1-eta, eta, eta]
            dx = [-1/hx, 1/hx, -1/hx, 1/hx]; dy = [-1/hy, -1/hy, 1/hy, 1/hy]
            B = np.zeros((3, 12)); G = np.zeros((2, 12)); V = np.zeros((3, 12))
            for k in range(4):
                ndx, ndy = dx[k]*ny[k], nx[k]*dy[k]
                G[0, 3*k] = ndx; G[1, 3*k] = ndy; G[0, 3*k+1] = 0.5*ny[k]; G[1, 3*k+2] = 0.5*nx[k]
                B[0, 3*k+1] = ndx; B[2, 3*k+1] = ndy; B[1, 3*k+2] = ndy; B[2, 3*k+2] = ndx
                for d in range(3): V[d, 3*k+d] = N[k]
            w = w1*w2*hx*hy
            ke += w*(B.T@D@B + G.T@S@G); me += w*(V.T@np.diag([m, I, I])@V)
    for k in range(4):
        lump[3*k] = m*hx*hy/4; lump[3*k+1] = lump[3*k+2] = I*hx*hy/4
    return ke, me, np.diag(lump)
def bloch(ke, me, kx, ky, hx, hy):
    ph = [1, np.exp(1j*kx*hx), np.exp(1j*ky*hy), np.exp(1j*(kx*hx+ky*hy))]
    P = np.zeros((12, 3), complex)
    for k in range(4):
        for d in range(3): P[3*k+d, d] = ph[k]
    K = P.conj().T@ke@P; M = P.conj().T@me@P
    lam = np.linalg.eigvals(np.linalg.solve(M, K)).real
    return np.sqrt(max(lam.min(), 0))
def exact(D, S, m, I, kx, ky):
    B = np.array([[0, 1j*kx, 0], [0, 0, 1j*ky], [0, 1j*ky, 1j*kx]]); G = np.array([[1j*kx, 1, 0], [1j*ky, 0, 1]])
    K = B.conj().T@D@B + G.conj().T@S@G
    lam = np.linalg.eigvals(np.linalg.solve(np.diag([m, I, I]), K)).real
    return np.sqrt(max(lam.min(), 0))
def k_at(D, S, m, I, f, th):
    lo, hi = 1e-3, 2000.0
    for _ in range(80):
        k = 0.5*(lo+hi)
        if exact(D, S, m, I, k*np.cos(th), k*np.sin(th)) < 2*np.pi*f: lo = k
        else: hi = k
    return k
hthick = float(sys.argv[1]) if len(sys.argv) > 1 else 0.009
over = float(sys.argv[2]) if len(sys.argv) > 2 else 0.5
grain = np.radians(40)
D, S, m, I = material(hthick, grain)
freqs = [1000, 2000, 3000, 4000, 6000, 8000, 10000, 12000]
print(f"plate of {hthick*1000:.0f} mm; wavelength (cm) along / across the grain:")
for f in freqs:
    print(f"  {f:6} Hz: {2*np.pi/k_at(D,S,m,I,f,grain)*100:5.1f} / {2*np.pi/k_at(D,S,m,I,f,grain+np.pi/2)*100:5.1f}")
for step in (0.02, 0.01, 0.005):
    ke, me, ml = element(step, step, D, S, m, I)
    print(f"step {step*100:.1f} cm: error in frequency, %, mean over the directions (worst), at {freqs} Hz")
    for name, mm in (("consistent", me), ("lumped", ml), (f"over {over}", ml + over*(ml - me))):
        row = []
        for f in freqs:
            errs = []
            for th in np.linspace(0, np.pi, 36, endpoint=False):
                k = k_at(D, S, m, I, f, th)
                errs.append(bloch(ke, mm, k*np.cos(th), k*np.sin(th), step, step)/(2*np.pi*f)-1)
            row.append(f"{100*np.mean(errs):+5.1f} ({100*max(errs, key=abs):+6.1f})")
        print(f"  {name:11} " + "  ".join(row))
