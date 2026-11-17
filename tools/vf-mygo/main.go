// vf-mygo: renderer alternativo da video-factory. Lê o mesmo data/edl.json + data/words.json do
// template Remotion e desenha cada frame na GPU (Metal) com o plugin render do MyGo, sem navegador.
//
//	vf-mygo -project <pasta> -formato yt-short -o out/video.mp4 [-from 0 -to N] [-still 80 -o f.png]
//
// Cobre: clipes image (Ken Burns + parallax), video (frames exatos via ffmpeg), text
// (title/lower/label/number), color, transições cut/fade/flash/slide e legendas karaokê/pop/plain.
// O áudio fica de fora, como no Remotion com --muted: o mix entra no `vf export --audio`.
package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"image"
	"image/png"
	"log"
	"math"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/egoist/mygo/plugins/render"
	"github.com/egoist/mygo/ui"
)

// ---------- EDL (mesmo contrato de skills/animacao/templates/edl.schema.md) ----------

type Pose struct {
	Scale  *float64 `json:"scale"`
	X      *float64 `json:"x"`
	Y      *float64 `json:"y"`
	Rotate *float64 `json:"rotate"`
}
type Motion struct {
	From *Pose  `json:"from"`
	To   *Pose  `json:"to"`
	Ease string `json:"ease"`
	// câmera com junção de trechos: from -> via (zoom-out) -> to (zoom-in), cada perna com o ease (padrão inOut)
	Via   *Pose    `json:"via"`
	ViaAt *float64 `json:"via_at"` // fração do clipe em que a câmera chega no via (padrão 0.5)
}
type LayerSrc struct {
	Src   string  `json:"src"`
	Depth float64 `json:"depth"`
}
type Transition struct {
	Kind string   `json:"kind"`
	Dur  *float64 `json:"dur"`
}
type Clip struct {
	ID           string      `json:"id"`
	Type         string      `json:"type"`
	Start        float64     `json:"start"`
	End          float64     `json:"end"`
	Layer        int         `json:"layer"`
	TransitionIn *Transition `json:"transition_in"`
	Motion       *Motion     `json:"motion"`
	Src          string      `json:"src"`
	Fit          string      `json:"fit"`
	Layers       []LayerSrc  `json:"layers"`
	Trim         float64     `json:"trim"`
	PlaybackRate float64     `json:"playbackRate"`
	Text         string      `json:"text"`
	Style        string      `json:"style"`
	Color        string      `json:"color"`
	Scene        *SceneSpec  `json:"scene"`
	end          float64     // com extensão para o fade do próximo
}
type Captions struct {
	Enabled       bool   `json:"enabled"`
	WordsPerPage  int    `json:"words_per_page"`
	Style         string `json:"style"`
}
type FontSpec struct {
	Family string `json:"family"`
	Src    string `json:"src"`
}
type EDL struct {
	FPS        float64   `json:"fps"`
	Font       *FontSpec `json:"font"`
	Duration   float64   `json:"duration"`
	Background string    `json:"background"`
	Clips      []Clip    `json:"clips"`
	Captions   *Captions `json:"captions"`
}
type Word struct {
	Word  string  `json:"word"`
	Start float64 `json:"start"`
	End   float64 `json:"end"`
}

type Safe struct{ Top, Bottom, Left, Right float32 }
type Fmt struct {
	W, H int
	Safe Safe
}

// espelho de formats.yaml / src/lib/formats.ts
var formats = map[string]Fmt{
	"yt-long":  {1920, 1080, Safe{0, 0, 0, 0}},
	"yt-short": {1080, 1920, Safe{140, 380, 40, 120}},
	"tiktok":   {1080, 1920, Safe{160, 480, 60, 140}},
}

// ---------- matemática (igual ao template Remotion) ----------

func clamp01(x float64) float64 { return math.Max(0, math.Min(1, x)) }

// bezier cúbica (x1,y1,x2,y2) como Easing.bezier do Remotion/CSS
func bezier(x1, y1, x2, y2 float64) func(float64) float64 {
	return func(t float64) float64 {
		if t <= 0 || t >= 1 {
			return t
		}
		u := t
		for i := 0; i < 12; i++ { // Newton em x(u) = t
			x := 3*(1-u)*(1-u)*u*x1 + 3*(1-u)*u*u*x2 + u*u*u - t
			dx := 3*(1-u)*(1-u)*x1 + 6*(1-u)*u*(x2-x1) + 3*u*u*(1-x2)
			if math.Abs(dx) < 1e-9 {
				break
			}
			u = clamp01(u - x/dx)
		}
		return 3*(1-u)*(1-u)*u*y1 + 3*(1-u)*u*u*y2 + u*u*u
	}
}

var easings = map[string]func(float64) float64{
	"linear": func(t float64) float64 { return t },
	"in":     func(t float64) float64 { return t * t * t },
	"out":    func(t float64) float64 { return 1 - math.Pow(1-t, 3) },
	"inOut":  bezier(0.45, 0, 0.55, 1),
}

func progress(frame, dur int, ease string) float64 {
	e, ok := easings[ease]
	if !ok {
		e = easings["linear"]
	}
	return e(clamp01(float64(frame) / math.Max(1, float64(dur-1))))
}

func lerp(a, b, p float64) float64 { return a + (b-a)*p }
func interp(x, x0, x1, y0, y1 float64) float64 {
	return lerp(y0, y1, clamp01((x-x0)/(x1-x0)))
}

func val(p *float64, d float64) float64 {
	if p == nil {
		return d
	}
	return *p
}

type pose struct{ scale, x, y, rot float64 }

func mix(p float64, a, b *Pose) pose {
	if a == nil {
		a = &Pose{}
	}
	if b == nil {
		b = &Pose{}
	}
	as, ax, ay, ar := val(a.Scale, 1), val(a.X, 0), val(a.Y, 0), val(a.Rotate, 0)
	return pose{lerp(as, val(b.Scale, as), p), lerp(ax, val(b.X, ax), p), lerp(ay, val(b.Y, ay), p), lerp(ar, val(b.Rotate, ar), p)}
}

// spring como o do Remotion (massa-mola-amortecedor saindo de 0 para 1), forma fechada
func spring(frame int, fps, damping, stiffness, mass float64) float64 {
	if frame <= 0 {
		return 0
	}
	t := float64(frame) / fps
	w0 := math.Sqrt(stiffness / mass)
	z := damping / (2 * math.Sqrt(stiffness*mass))
	if z < 1 {
		wd := w0 * math.Sqrt(1-z*z)
		return 1 - math.Exp(-z*w0*t)*(math.Cos(wd*t)+(z*w0/wd)*math.Sin(wd*t))
	}
	return 1 - math.Exp(-w0*t)*(1+w0*t)
}

// ---------- assets ----------

type assets struct {
	root    string
	bitmaps map[string]*ui.Bitmap
	videos  map[string]*videoFeed
}

func (a *assets) bitmap(src string) *ui.Bitmap {
	if b, ok := a.bitmaps[src]; ok {
		return b
	}
	data, err := os.ReadFile(filepath.Join(a.root, src))
	if err != nil {
		log.Fatalf("asset %s: %v", src, err)
	}
	b, err := ui.DecodeBitmap(data)
	if err != nil {
		log.Fatalf("decodificar %s: %v", src, err)
	}
	a.bitmaps[src] = b
	return b
}

// ---------- componentes ----------

const accent = "#ffd23f"

type ctx struct {
	font string
	// lineEm: altura natural da linha (ascent+descent) em em, para imitar a meia entrelinha do CSS
	lineEm float32
	fps  float64
	w, h float32
	u    float32
	safe Safe
	a    *assets
}

// halfLeading: o CSS centraliza a área de conteúdo (ascent+descent) na linha; o mygo alinha pelo
// topo. Desloca o texto para cima por (natural - lineHeight)/2.
func (k *ctx) halfLeading(lineHeight, size float32) float32 {
	if k.lineEm <= 0 {
		return 0
	}
	return -(k.lineEm - lineHeight) / 2 * size
}

// txt cria um Text já com a fonte do vídeo (edl.json > font)
func (k *ctx) txt(c *ui.Context, s string) ui.Element {
	t := ui.Text(c, s)
	if k.font != "" {
		t = t.Font(k.font)
	}
	return t
}

func (k *ctx) sec(s float64) int { return int(math.Round(s * k.fps)) }

func (k *ctx) kenburns(c *ui.Context, cl *Clip, frame, dur int) {
	m := cl.Motion
	if m == nil {
		one, ten := 1.0, 1.1
		m = &Motion{From: &Pose{Scale: &one}, To: &Pose{Scale: &ten}, Ease: "linear"}
	}
	ease := m.Ease
	if ease == "" {
		ease = "linear"
	}
	b := mix(progress(frame, dur, ease), m.From, m.To)
	layers := cl.Layers
	if len(layers) == 0 && cl.Src != "" {
		layers = []LayerSrc{{Src: cl.Src}}
	}
	fit := ui.Cover
	if cl.Fit == "contain" {
		fit = ui.Contain
	}
	render.Layer(c).Clip().Children(func() {
		for _, l := range layers {
			amp := 1 + l.Depth*0.8
			s := float32(1 + (b.scale-1)*amp)
			tx := float32(b.x * amp * float64(k.w))
			ty := float32(b.y * amp * float64(k.h))
			bm := k.a.bitmap(l.Src)
			// como no Remotion: o transform vai no quadro inteiro, origem no centro.
			// CSS "translate() scale() rotate()" = rotaciona, escala e depois translada
			render.Layer(c).TransformOrigin(0.5, 0.5).Rotate(float32(b.rot)).Scale(s, s).Translate(tx, ty).Children(func() {
				ui.Image(c, bm).Size(k.w, k.h).Fit(fit)
			})
		}
	})
}

func (k *ctx) video(c *ui.Context, cl *Clip, frame, dur int) {
	rate := cl.PlaybackRate
	if rate == 0 {
		rate = 1
	}
	vf := k.a.videos[cl.ID]
	src := int(math.Round(cl.Trim*k.fps + float64(frame)*rate))
	img := vf.frame(src)
	if img == nil {
		return
	}
	var m *Motion = cl.Motion
	p := pose{1, 0, 0, 0}
	if m != nil {
		p = mix(progress(frame, dur, m.Ease), m.From, m.To)
	}
	fit := ui.Cover
	if cl.Fit == "contain" {
		fit = ui.Contain
	}
	render.Layer(c).Clip().Children(func() {
		render.Layer(c).TransformOrigin(0.5, 0.5).Scale(float32(p.scale), float32(p.scale)).
			Translate(float32(p.x)*k.w, float32(p.y)*k.h).Children(func() {
			ui.Image(c, vf.bm).Size(k.w, k.h).Fit(fit)
		})
	})
}

var digits = regexp.MustCompile(`[\d.,-]+`)
var grouped = regexp.MustCompile(`\d[.,]\d{3}`)

func thousands(n int) string {
	s := strconv.Itoa(n)
	neg := strings.HasPrefix(s, "-")
	s = strings.TrimPrefix(s, "-")
	var out []string
	for len(s) > 3 {
		out = append([]string{s[len(s)-3:]}, out...)
		s = s[:len(s)-3]
	}
	out = append([]string{s}, out...)
	r := strings.Join(out, ".")
	if neg {
		r = "-" + r
	}
	return r
}

func (k *ctx) text(c *ui.Context, cl *Clip, frame, dur int) {
	u := k.u
	pop := float32(spring(frame, k.fps, 14, 180, 0.6))
	out := float32(interp(float64(frame), float64(dur-6), float64(dur), 1, 0))
	color := ui.Hex("#ffffff")
	if cl.Color != "" {
		color = ui.Hex(cl.Color)
	}
	s := k.safe
	switch cl.Style {
	case "number":
		target, _ := strconv.ParseFloat(strings.NewReplacer(".", "", ",", ".").Replace(digits.FindString(cl.Text)), 64)
		if !grouped.MatchString(cl.Text) {
			target, _ = strconv.ParseFloat(strings.ReplaceAll(digits.FindString(cl.Text), ",", "."), 64)
		}
		v := interp(float64(frame), 0, math.Min(float64(dur), k.fps*1.2), 0, target)
		n := int(math.Round(v))
		shown := strconv.Itoa(n)
		if grouped.MatchString(cl.Text) {
			shown = thousands(n)
		}
		shown = digits.ReplaceAllLiteralString(cl.Text, shown)
		sc := 0.6 + 0.4*pop
		render.Layer(c).Justify(ui.Center).AlignItems(ui.Center).Opacity(out).Children(func() {
			k.txt(c, shown).FontSize(190*u).FontWeight(900).TextColor(color).TextStroke(10*u, ui.Hex("#000000")).Scale(sc, sc)
		})
	case "lower":
		x := (1 - pop) * -k.w * 0.6
		render.Layer(c).Justify(ui.End).AlignItems(ui.Start).Padding(0, s.Right+60*u, s.Bottom+140*u, s.Left+60*u).Opacity(out).Children(func() {
			ui.Row(c).Translate(x, 0).Children(func() {
				ui.Box(c).Width(10 * u).Background(ui.Hex(accent))
				k.txt(c, cl.Text).FontSize(54*u).FontWeight(800).TextColor(color).Padding(18*u, 30*u).Background(ui.RGBA(0, 0, 0, 0.72))
			})
		})
	case "label":
		render.Layer(c).Justify(ui.Start).AlignItems(ui.Start).Padding(s.Top+60*u, s.Right+60*u, 0, s.Left+60*u).Opacity(out).Children(func() {
			k.txt(c, cl.Text).FontSize(44*u).FontWeight(900).TextColor(ui.Hex("#111111")).Background(ui.Hex(accent)).
				Padding(10*u, 22*u).Radius(10*u).Scale(pop, pop)
		})
	default: // title
		sc := 0.7 + 0.3*pop
		render.Layer(c).Justify(ui.Center).AlignItems(ui.Center).Padding(s.Top, s.Right+60*u, s.Bottom, s.Left+60*u).Opacity(out).Children(func() {
			k.txt(c, strings.ToUpper(cl.Text)).FontSize(120*u).FontWeight(900).LineHeight(1.05).TextAlign(ui.Center).
				TextColor(color).TextStroke(8*u, ui.Hex("#000000")).Scale(sc, sc).Translate(0, (1-pop)*40*u)
		})
	}
}

func (k *ctx) clip(c *ui.Context, cl *Clip, frame, dur int) {
	tf := 0.25
	if cl.TransitionIn != nil && cl.TransitionIn.Dur != nil {
		tf = *cl.TransitionIn.Dur
	}
	tfr := math.Max(1, math.Round(tf*k.fps))
	kind := ""
	if cl.TransitionIn != nil {
		kind = cl.TransitionIn.Kind
	}
	op := float32(1)
	if kind == "fade" {
		op = float32(interp(float64(frame), 0, tfr, 0, 1))
	}
	slide := float32(0)
	if kind == "slide" {
		slide = float32((1 - easings["out"](clamp01(float64(frame)/tfr))) * float64(k.w))
	}
	flash := float32(0)
	if kind == "flash" {
		flash = float32(interp(float64(frame), 0, tfr, 1, 0))
	}
	render.Layer(c).Opacity(op).Translate(slide, 0).Children(func() {
		switch cl.Type {
		case "image":
			k.kenburns(c, cl, frame, dur)
		case "video":
			k.video(c, cl, frame, dur)
		case "text":
			k.text(c, cl, frame, dur)
		case "color":
			render.Layer(c).Background(ui.Hex(cl.Color))
		case "scene":
			k.scene(c, cl, frame, dur)
		}
		if flash > 0 {
			render.Layer(c).Background(ui.Hex("#ffffff")).Opacity(flash)
		}
	})
}

var endPunct = regexp.MustCompile(`[.!?…]$`)

func paginate(words []Word, n int) [][]Word {
	var pages [][]Word
	var cur []Word
	for i, w := range words {
		cur = append(cur, w)
		last := i+1 >= len(words)
		gap := !last && words[i+1].Start-w.End > 0.5
		if len(cur) >= n || endPunct.MatchString(w.Word) || gap || last {
			pages = append(pages, cur)
			cur = nil
		}
	}
	return pages
}

func (k *ctx) captions(c *ui.Context, frame int, pages [][]Word, cfg *Captions) {
	t := float64(frame) / k.fps
	var page []Word
	for i, p := range pages {
		end := p[len(p)-1].End + 0.4
		if i+1 < len(pages) {
			end = pages[i+1][0].Start
		}
		if t >= p[0].Start-0.05 && t < end {
			page = p
			break
		}
	}
	if page == nil {
		return
	}
	u := k.u
	vertical := k.h > k.w
	bottom := 110 * u
	size := 64 * u
	if vertical {
		bottom = k.safe.Bottom + 120*u
		size = 86 * u
	}
	render.Layer(c).Justify(ui.End).AlignItems(ui.Center).Padding(0, k.safe.Right+40*u, bottom, k.safe.Left+40*u).Children(func() {
		ui.Row(c).Wrap().Justify(ui.Center).GapX(0.32 * size).GapY(0).Children(func() {
			for _, w := range page {
				active := t >= w.Start && t < w.End+0.05
				said := t >= w.Start
				pop := float32(1)
				if cfg.Style == "pop" {
					if !said {
						continue
					}
					pop = float32(spring(frame-int(math.Round(w.Start*k.fps)), k.fps, 12, 220, 1))
				}
				col := ui.Hex("#ffffff")
				sc := pop
				if active && cfg.Style != "plain" {
					col = ui.Hex(accent)
				}
				if active && cfg.Style == "karaoke" {
					sc *= 1.06
				}
				k.txt(c, strings.ToUpper(w.Word)).FontSize(size).FontWeight(900).FixedLineHeight(1.1*size).Height(1.1*size).
					Translate(0, k.halfLeading(1.1, size)).
					TextColor(col).TextStroke(10*u, ui.Hex("#000000")).Scale(sc, sc)
			}
		})
	})
}

func main() {
	project := flag.String("project", ".", "pasta do projeto (data/, public/)")
	formato := flag.String("formato", "yt-short", "yt-long | yt-short | tiktok")
	outPath := flag.String("o", "out/video_mygo.mp4", "saída .mp4/.mov ou .png com -still")
	from := flag.Int("from", 0, "primeiro frame")
	to := flag.Int("to", 0, "último frame (exclusivo; 0 = fim)")
	still := flag.Int("still", -1, "renderiza só este frame em PNG")
	renderer := flag.String("renderer", "auto", "auto | gpu | cpu")
	bitrate := flag.Int("bitrate", 16_000_000, "bits/s do H.264 (encoder de hardware)")
	scale := flag.Float64("scale", 1, "multiplica a resolução (1.3333 = 1440x2560)")
	flag.Parse()

	f, ok := formats[*formato]
	if !ok {
		log.Fatalf("formato desconhecido: %s", *formato)
	}
	var edl EDL
	mustJSON(filepath.Join(*project, "data/edl.json"), &edl)
	var words []Word
	_ = readJSON(filepath.Join(*project, "data/words.json"), &words)
	if edl.FPS == 0 {
		edl.FPS = 30
	}

	a := &assets{root: filepath.Join(*project, "public"), bitmaps: map[string]*ui.Bitmap{}, videos: map[string]*videoFeed{}}
	k := &ctx{fps: edl.FPS, w: float32(f.W), h: float32(f.H), safe: f.Safe, a: a}
	if edl.Font != nil {
		data, err := os.ReadFile(filepath.Join(a.root, edl.Font.Src))
		if err != nil {
			log.Fatalf("fonte: %v", err)
		}
		if err := ui.RegisterFont(data, edl.Font.Family); err != nil {
			log.Fatalf("registrar fonte: %v", err)
		}
		k.font = edl.Font.Family
		k.lineEm = fontLineEm(data)
	}
	k.u = float32(math.Min(float64(f.W), float64(f.H)) / 1080)

	clips := append([]Clip(nil), edl.Clips...)
	sort.SliceStable(clips, func(i, j int) bool {
		if clips[i].Layer != clips[j].Layer {
			return clips[i].Layer < clips[j].Layer
		}
		return clips[i].Start < clips[j].Start
	})
	// estende o clipe de baixo para cobrir o fade do próximo (igual ao Main.tsx)
	for i := range clips {
		clips[i].end = clips[i].End
		for _, n := range clips {
			if n.Layer == clips[i].Layer && math.Abs(n.Start-clips[i].End) < 1e-3 && n.TransitionIn != nil && n.TransitionIn.Kind == "fade" {
				d := 0.25
				if n.TransitionIn.Dur != nil {
					d = *n.TransitionIn.Dur
				}
				clips[i].end += d
			}
		}
	}
	t0 := time.Now()
	for i := range clips {
		cl := &clips[i]
		switch cl.Type {
		case "image":
			if cl.Src != "" {
				a.bitmap(cl.Src)
			}
			for _, l := range cl.Layers {
				a.bitmap(l.Src)
			}
		case "scene":
			k.prepareScene(cl, words)
		case "video":
			if cl.ID == "" {
				cl.ID = fmt.Sprintf("video-%d", i)
			}
			a.videos[cl.ID] = newVideoFeed(filepath.Join(a.root, cl.Src), f.W, f.H, edl.FPS)
		}
	}
	load := time.Since(t0)
	pages := [][]Word{}
	capOn := edl.Captions != nil && edl.Captions.Enabled && len(words) > 0
	if capOn {
		n := edl.Captions.WordsPerPage
		if n <= 0 {
			n = 3
		}
		pages = paginate(words, n)
	}

	total := int(math.Max(1, math.Ceil(edl.Duration*edl.FPS)))
	bg := ui.Hex("#000000")
	if edl.Background != "" {
		bg = ui.Hex(edl.Background)
	}
	comp := render.Composition{
		Width: int(math.Round(float64(f.W) * *scale)), Height: int(math.Round(float64(f.H) * *scale)),
		FPS: edl.FPS, Frames: total, Scale: float32(*scale), Background: bg, Dark: true,
		View: func(c *ui.Context, fr render.Frame) {
			for i := range clips {
				cl := &clips[i]
				s := k.sec(cl.Start)
				d := max(1, k.sec(cl.end)-s)
				fr.Sequence(c, s, d, func(g render.Frame) { k.clip(c, cl, g.Index, d) })
			}
			if capOn {
				k.captions(c, fr.Index, pages, edl.Captions)
			}
		},
	}
	ctxb := context.Background()
	if *still >= 0 {
		img, err := render.Still(comp, *still, render.Options{Renderer: pickRenderer(*renderer)})
		if err != nil {
			log.Fatal(err)
		}
		w, err := os.Create(*outPath)
		if err != nil {
			log.Fatal(err)
		}
		defer w.Close()
		if err := png.Encode(w, img); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("still %d -> %s (%v)\n", *still, *outPath, time.Since(t0).Round(time.Millisecond))
		return
	}
	o := render.Options{From: *from, To: *to, Renderer: pickRenderer(*renderer), Codec: render.H264, Bitrate: *bitrate,
		Progress: func(p render.Progress) {
			if p.Done%30 == 0 || p.Done == p.Frames {
				fmt.Fprintf(os.Stderr, "\r%s: %d/%d frames, %.0f fps", p.Renderer, p.Done, p.Frames, float64(p.Done)/p.Elapsed.Seconds())
			}
		}}
	_ = os.MkdirAll(filepath.Dir(*outPath), 0o755)
	t1 := time.Now()
	if err := render.Video(ctxb, comp, *outPath, o); err != nil {
		log.Fatal(err)
	}
	for _, v := range a.videos {
		v.close()
	}
	el := time.Since(t1)
	fmt.Fprintf(os.Stderr, "\n%s: %d frames %dx%d em %v (%.0f fps; carga de assets %v)\n", *outPath, total, comp.Width, comp.Height,
		el.Round(time.Millisecond), float64(total)/el.Seconds(), load.Round(time.Millisecond))
}

func pickRenderer(s string) render.Renderer {
	switch s {
	case "gpu":
		return render.GPU
	case "cpu":
		return render.CPU
	}
	return render.Auto
}

func readJSON(p string, v any) error {
	b, err := os.ReadFile(p)
	if err != nil {
		return err
	}
	return json.Unmarshal(b, v)
}

func mustJSON(p string, v any) {
	if err := readJSON(p, v); err != nil {
		log.Fatalf("%s: %v", p, err)
	}
}

var _ image.Image // (png/still)
