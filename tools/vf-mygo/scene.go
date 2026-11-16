package main

// Clipe "scene": animação cutout 2D. Cenário (plate) + câmera + atores-sprite com keyframes,
// estados (expressões, ciclos de caminhada), piscada automática, idle, entrada/saída e sombra de contato.
// A matemática espelha skills/animacao/templates/remotion-2d/src/lib/anim.ts: os dois renderers dão o mesmo resultado.

import (
	"math"
	"path/filepath"
	"sort"

	"github.com/egoist/mygo/plugins/render"
	"github.com/egoist/mygo/ui"
)

type SpriteState struct {
	Frames []string `json:"frames"`
	FPS    float64  `json:"fps"`
	Blink  string   `json:"blink"`
}
type Sprite struct {
	W      float64                 `json:"w"`
	H      float64                 `json:"h"`
	Anchor [2]float64              `json:"anchor"`
	States map[string]*SpriteState `json:"states"`
	dir    string
	first  string
}

type Key struct {
	T       *float64 `json:"t"`
	AtWord  *int     `json:"at_word"`
	X       *float64 `json:"x"`
	Y       *float64 `json:"y"`
	Scale   *float64 `json:"scale"`
	Rot     *float64 `json:"rot"`
	Opacity *float64 `json:"opacity"`
	State   *string  `json:"state"`
	Flip    *bool    `json:"flip"`
	Ease    string   `json:"ease"`
	t       float64
}
type Move struct {
	Kind   string   `json:"kind"`
	At     *float64 `json:"at"`
	AtWord *int     `json:"at_word"`
	Dur    float64  `json:"dur"`
	t      float64
}
type Actor struct {
	ID      string  `json:"id"`
	Sprite  string  `json:"sprite"`
	H       float64 `json:"h"`
	Depth   float64 `json:"depth"`
	X       float64 `json:"x"`
	Y       float64 `json:"y"`
	Scale   float64 `json:"scale"`
	Rot     float64 `json:"rot"`
	Opacity float64 `json:"opacity"`
	State   string  `json:"state"`
	Flip    bool    `json:"flip"`
	Keys    []Key   `json:"keys"`
	Idle    string  `json:"idle"`
	Blink   *bool   `json:"blink"`
	Shadow  bool    `json:"shadow"`
	// ClipY: recorta o ator abaixo desta linha (fração da altura do quadro): atrás de mesa, muro, janela
	ClipY float64 `json:"clip_y"`
	Enter *Move   `json:"enter"`
	Exit  *Move   `json:"exit"`
	sp    *Sprite
	seed  float64
	swaps []float64 // tempos de troca de estado (piscada cobre a troca)
}
type SceneBG struct {
	Src   string `json:"src"`
	Color string `json:"color"`
	Fit   string `json:"fit"`
}
type SceneSpec struct {
	BG     *SceneBG `json:"bg"`
	Camera *Motion  `json:"camera"`
	Actors []Actor  `json:"actors"`
}

// hh: mesmo hash do anim.ts (determinístico, igual em TS)
func hh(i, s float64) float64 {
	v := math.Sin(i*127.1+s*311.7) * 43758.5453
	return v - math.Floor(v)
}

func strHash(s string) float64 {
	h := 0.0
	for _, r := range s {
		h = math.Mod(h*31+float64(r), 9973)
	}
	return h
}

var sprites = map[string]*Sprite{}

func loadSprite(root, dir string) *Sprite {
	if sp, ok := sprites[dir]; ok {
		return sp
	}
	var sp Sprite
	mustJSON(filepath.Join(root, dir, "sprite.json"), &sp)
	sp.dir = dir
	if sp.Anchor == [2]float64{} {
		sp.Anchor = [2]float64{0.5, 0.975}
	}
	names := make([]string, 0, len(sp.States))
	for k := range sp.States {
		names = append(names, k)
	}
	sort.Strings(names)
	if _, ok := sp.States["neutral"]; ok {
		sp.first = "neutral"
	} else if len(names) > 0 {
		sp.first = names[0]
	}
	sprites[dir] = &sp
	return &sp
}

// prepara: resolve at_word, ordena keys, carrega bitmaps
func (k *ctx) prepareScene(cl *Clip, words []Word) {
	if cl.Scene == nil {
		return
	}
	wt := func(w *int, t *float64, def float64) float64 {
		if w != nil && len(words) > 0 {
			i := max(0, min(len(words)-1, *w))
			return words[i].Start - cl.Start - 2/k.fps // reação 2 frames antes da palavra
		}
		if t != nil {
			return *t
		}
		return def
	}
	if cl.Scene.BG != nil && cl.Scene.BG.Src != "" {
		k.a.bitmap(cl.Scene.BG.Src)
	}
	for i := range cl.Scene.Actors {
		a := &cl.Scene.Actors[i]
		a.sp = loadSprite(k.a.root, a.Sprite)
		if a.Scale == 0 {
			a.Scale = 1
		}
		if a.Opacity == 0 {
			a.Opacity = 1
		}
		if a.State == "" {
			a.State = a.sp.first
		}
		if a.H == 0 {
			a.H = 0.5
		}
		a.seed = strHash(a.ID + a.Sprite)
		for j := range a.Keys {
			a.Keys[j].t = wt(a.Keys[j].AtWord, a.Keys[j].T, 0)
		}
		sort.SliceStable(a.Keys, func(x, y int) bool { return a.Keys[x].t < a.Keys[y].t })
		for _, kk := range a.Keys {
			if kk.State != nil {
				a.swaps = append(a.swaps, kk.t)
			}
		}
		if a.Enter != nil {
			a.Enter.t = wt(a.Enter.AtWord, a.Enter.At, 0)
			if a.Enter.Dur == 0 {
				a.Enter.Dur = 0.35
			}
		}
		if a.Exit != nil {
			a.Exit.t = wt(a.Exit.AtWord, a.Exit.At, cl.End-cl.Start)
			if a.Exit.Dur == 0 {
				a.Exit.Dur = 0.3
			}
		}
		for _, st := range a.sp.States {
			for _, f := range st.Frames {
				k.a.bitmap(filepath.Join(a.Sprite, f))
			}
			if st.Blink != "" {
				k.a.bitmap(filepath.Join(a.Sprite, st.Blink))
			}
		}
	}
}

// valor numérico animado: base até a 1ª key que define o campo; entre keys, lerp com o ease da key de destino
func numAt(t, base float64, keys []Key, get func(Key) *float64) float64 {
	v, pt := base, math.Inf(-1)
	for _, kk := range keys {
		p := get(kk)
		if p == nil {
			continue
		}
		if t >= kk.t {
			v, pt = *p, kk.t
			continue
		}
		if math.IsInf(pt, -1) {
			return v // antes da 1ª key: segura o valor base
		}
		ease := kk.Ease
		if ease == "" {
			ease = "inOut"
		}
		e, ok := easings[ease]
		if !ok {
			e = easings["inOut"]
		}
		return lerp(v, *p, e(clamp01((t-pt)/math.Max(1e-6, kk.t-pt))))
	}
	return v
}

func backOut(u float64) float64 { // ease com leve passada (drop/slide)
	c1 := 1.4
	c3 := c1 + 1
	return 1 + c3*math.Pow(u-1, 3) + c1*math.Pow(u-1, 2)
}

type actorPose struct {
	x, y, sx, sy, rot, op float64
	flip                  bool
	img                   string
	visible               bool
}

func (k *ctx) actorAt(a *Actor, t float64, frame int) actorPose {
	p := actorPose{visible: true}
	p.x = numAt(t, a.X, a.Keys, func(q Key) *float64 { return q.X })
	p.y = numAt(t, a.Y, a.Keys, func(q Key) *float64 { return q.Y })
	s := numAt(t, a.Scale, a.Keys, func(q Key) *float64 { return q.Scale })
	p.rot = numAt(t, a.Rot, a.Keys, func(q Key) *float64 { return q.Rot })
	p.op = numAt(t, a.Opacity, a.Keys, func(q Key) *float64 { return q.Opacity })
	state, flip := a.State, a.Flip
	for _, kk := range a.Keys {
		if kk.t <= t {
			if kk.State != nil {
				state = *kk.State
			}
			if kk.Flip != nil {
				flip = *kk.Flip
			}
		}
	}
	p.flip = flip
	sx, sy := s, s
	// entrada / saída
	for _, mv := range []*Move{a.Enter, a.Exit} {
		if mv == nil {
			continue
		}
		in := mv == a.Enter
		var u float64
		if in {
			if t < mv.t {
				p.visible = false
				return p
			}
			u = clamp01((t - mv.t) / mv.Dur)
		} else {
			if t >= mv.t+mv.Dur {
				p.visible = false
				return p
			}
			if t < mv.t {
				continue
			}
			u = 1 - clamp01((t-mv.t)/mv.Dur)
		}
		switch mv.Kind {
		case "pop":
			var g float64
			if in {
				g = spring(int(math.Round((t-mv.t)*k.fps)), k.fps, 11, 210, 0.7)
			} else {
				g = easings["in"](u)
			}
			sx *= g
			sy *= g
		case "slide-left":
			p.x -= (1 - backOut(u)) * 0.75
		case "slide-right":
			p.x += (1 - backOut(u)) * 0.75
		case "drop":
			p.y -= (1 - backOut(u)) * 0.7
		case "rise":
			p.y += (1 - easings["out"](u)) * 0.5
		default: // fade
			p.op *= u
		}
	}
	// idle (fase por ator)
	ph := hh(a.seed, 3)
	switch a.Idle {
	case "", "breathe":
		b := math.Sin(2 * math.Pi * (t/2.6 + ph))
		sy *= 1 + 0.012*b
		sx *= 1 - 0.006*b
	case "bob":
		p.y -= 0.008 * math.Abs(math.Sin(math.Pi*(t/0.45+ph)))
	case "sway":
		p.rot += 1.6 * math.Sin(2*math.Pi*(t/3.2+ph))
	case "float":
		p.y += 0.006 * math.Sin(2*math.Pi*(t/2.2+ph))
	case "jitter": // nervoso / tremendo
		p.x += 0.0015 * math.Sin(t*47+ph*9)
		p.rot += 0.8 * math.Sin(t*39+ph*5)
	}
	p.sx, p.sy = sx, sy
	// frame do estado (ciclos) + piscada
	st := a.sp.States[state]
	if st == nil {
		st = a.sp.States[a.sp.first]
	}
	if st == nil || len(st.Frames) == 0 {
		p.visible = false
		return p
	}
	idx := 0
	if len(st.Frames) > 1 {
		fps := st.FPS
		if fps == 0 {
			fps = 12
		}
		idx = int(math.Floor(t*fps)) % len(st.Frames)
	}
	p.img = filepath.Join(a.Sprite, st.Frames[idx])
	if st.Blink != "" && (a.Blink == nil || *a.Blink) && k.blinking(a, frame) {
		p.img = filepath.Join(a.Sprite, st.Blink)
	}
	return p
}

// piscada: a cada 0,5-2,3 s por 4 frames, e uma em cada troca de estado
func (k *ctx) blinking(a *Actor, f int) bool {
	for _, ts := range a.swaps {
		d := f - int(math.Round(ts*k.fps)) + 1
		if d >= 0 && d < 3 {
			return true
		}
	}
	s := int(math.Round(-60 * hh(a.seed, 9)))
	for n := 0; s <= f && n < 2000; n++ {
		if d := f - s; d >= 0 && d < 4 {
			return true
		}
		s += int(math.Round(15 + 54*hh(a.seed*31+float64(n), 13)))
	}
	return false
}

func (k *ctx) camAt(m *Motion, frame, dur int) pose {
	if m == nil {
		return pose{1, 0, 0, 0}
	}
	ease := m.Ease
	if ease == "" {
		ease = "linear"
	}
	return mix(progress(frame, dur, ease), m.From, m.To)
}

func (k *ctx) scene(c *ui.Context, cl *Clip, frame, dur int) {
	sc := cl.Scene
	if sc == nil {
		return
	}
	t := float64(frame) / k.fps
	cam := k.camAt(sc.Camera, frame, dur)
	// cada camada recebe a câmera com parallax pela profundidade (fundo = 0)
	withCam := func(depth float64, body func()) {
		amp := 1 + depth*0.5
		s := float32(1 + (cam.scale-1)*amp)
		render.Layer(c).TransformOrigin(0.5, 0.5).Rotate(float32(cam.rot)).Scale(s, s).
			Translate(float32(cam.x*amp)*k.w, float32(cam.y*amp)*k.h).Children(body)
	}
	render.Layer(c).Clip().Children(func() {
		if sc.BG != nil {
			withCam(0, func() {
				if sc.BG.Color != "" {
					render.Layer(c).Background(ui.Hex(sc.BG.Color))
				}
				if sc.BG.Src != "" {
					fit := ui.Cover
					if sc.BG.Fit == "contain" {
						fit = ui.Contain
					}
					ui.Image(c, k.a.bitmap(sc.BG.Src)).Size(k.w, k.h).Fit(fit)
				}
			})
		}
		for i := range sc.Actors {
			a := &sc.Actors[i]
			p := k.actorAt(a, t, frame)
			if !p.visible || p.op <= 0.001 {
				continue
			}
			ph := float32(a.H) * k.h
			pw := ph * float32(a.sp.W/a.sp.H)
			ax, ay := float32(a.sp.Anchor[0]), float32(a.sp.Anchor[1])
			left := float32(p.x)*k.w - ax*pw
			top := float32(p.y)*k.h - ay*ph
			fx := float32(1)
			if p.flip {
				fx = -1
			}
			bm := k.a.bitmap(p.img)
			withCam(a.Depth, func() {
				body := func() {
					if a.Shadow {
						sw := pw * 0.42 * float32(math.Abs(p.sx))
						shh := sw * 0.22
						ui.Box(c).Absolute().Left(float32(p.x)*k.w-sw/2).Top(float32(p.y)*k.h-shh/2).Size(sw, shh).
							Radius(shh / 2).Background(ui.RGBA(36, 23, 58, 0.32*float32(p.op))).Blur(4)
					}
					ui.Image(c, bm).Absolute().Left(left).Top(top).Size(pw, ph).Opacity(float32(p.op)).
						TransformOrigin(ax, ay).Scale(fx*float32(p.sx), float32(p.sy)).Rotate(float32(p.rot))
				}
				if a.ClipY > 0 {
					ui.Box(c).Absolute().Left(0).Top(0).Size(k.w, float32(a.ClipY)*k.h).Clip().Children(body)
				} else {
					body()
				}
			})
		}
	})
}
