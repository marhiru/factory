package main

import (
	"golang.org/x/image/font/sfnt"
	"golang.org/x/image/math/fixed"
)

type fixed26 = fixed.Int26_6

// fontLineEm devolve (ascent+descent)/unitsPerEm da fonte, ou 0 se não der para ler.
func fontLineEm(data []byte) float32 {
	f, err := sfnt.Parse(data)
	if err != nil {
		return 0
	}
	upem := float64(f.UnitsPerEm())
	var b sfnt.Buffer
	m, err := f.Metrics(&b, toFixed(upem), 0)
	if err != nil || upem == 0 {
		return 0
	}
	return float32(float64(m.Ascent+m.Descent) / 64 / upem)
}

func toFixed(v float64) fixed26 { return fixed26(v * 64) }
