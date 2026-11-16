#!/usr/bin/env python3
"""Exemplo de cenas cutout com o sprite `exemplo` (templates/sprites/registry.ts).
Copie para roteiro/cenas.py e troque pelas cenas do seu roteiro. uso: python3 roteiro/cenas.py
Regras: uma cena por plano; toda ação cai numa palavra (w(i) ou find("palavra")); câmera nunca parada."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vfscene import *

init()
fim = W[-1]["end"]
meio = fim / 2

# plano 1: o boneco surge e se assusta na 3ª palavra
scene(0.0, meio, [
    A("heroi", "exemplo", 0.5, 0.88, 0.55, "neutro", enter={"kind": "rise", "T": 0.05, "dur": 0.5},
      keys=[st(w(min(2, len(W) - 1)), "susto"), {"T": w(min(2, len(W) - 1)), "scale": 1.0},
            {"T": w(min(2, len(W) - 1)) + 0.18, "scale": 1.06, "ease": "out"}, {"T": w(min(2, len(W) - 1)) + 0.4, "scale": 1.0}]),
], bg={"color": "#2a3550"}, cam=push(1.0, 1.06))

# plano 2: ele atravessa acenando (estado em ciclo + keys de x) e sai
scene(meio, fim + 0.4, [
    A("heroi", "exemplo", 0.2, 0.9, 0.5, "aceno", keys=[{"T": meio, "x": 0.2}, {"T": fim, "x": 0.8, "ease": "inOut"}],
      exit={"kind": "pop", "T": fim}),
], bg={"color": "#3a2a50"}, cam=pan(0.03), trans=XF)

save()
