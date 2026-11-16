package main

// Clipes de vídeo com frame exato: um ffmpeg por clipe decodifica em RGBA no tamanho do quadro
// (preservando o aspecto) e entrega frames em ordem. O render pede frames crescentes; um salto para
// trás ou grande para frente reabre o ffmpeg com -ss no frame pedido.

import (
	"bufio"
	"fmt"
	"image"
	"io"
	"log"
	"os/exec"

	"github.com/egoist/mygo/ui"
)

type videoFeed struct {
	path       string
	fps        float64
	w, h       int // tamanho do frame decodificado
	cmd        *exec.Cmd
	rd         *bufio.Reader
	next       int // índice do próximo frame que o pipe entrega
	buf        [2]*image.RGBA
	cur        int
	bm         *ui.Bitmap
	shown      int
	eof        bool
}

func newVideoFeed(path string, maxW, maxH int, fps float64) *videoFeed {
	// tamanho de saída: cabe em maxW x maxH preservando aspecto (o Fit cover/contain do ui faz o resto)
	out, err := exec.Command("ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", path).Output()
	if err != nil {
		log.Fatalf("ffprobe %s: %v", path, err)
	}
	var sw, sh int
	fmt.Sscanf(string(out), "%dx%d", &sw, &sh)
	if sw == 0 || sh == 0 {
		log.Fatalf("vídeo sem tamanho: %s", path)
	}
	// cobre o quadro (cover) sem passar de 1,0x a resolução de saída
	sc := max(float64(maxW)/float64(sw), float64(maxH)/float64(sh))
	w, h := int(float64(sw)*sc)&^1, int(float64(sh)*sc)&^1
	v := &videoFeed{path: path, fps: fps, w: w, h: h, shown: -1}
	for i := range v.buf {
		v.buf[i] = image.NewRGBA(image.Rect(0, 0, w, h))
	}
	v.bm = ui.NewBitmap(v.buf[0])
	return v
}

func (v *videoFeed) open(at int) {
	v.close()
	args := []string{"-hide_banner", "-loglevel", "error", "-nostdin", "-threads", "0"}
	if at > 0 {
		args = append(args, "-ss", fmt.Sprintf("%.6f", float64(at)/v.fps))
	}
	args = append(args, "-i", v.path, "-an", "-sn", "-map", "0:v:0",
		"-vf", fmt.Sprintf("fps=%g,scale=%d:%d:flags=bicubic,format=rgba", v.fps, v.w, v.h), "-f", "rawvideo", "-")
	v.cmd = exec.Command("ffmpeg", args...)
	pipe, err := v.cmd.StdoutPipe()
	if err != nil {
		log.Fatal(err)
	}
	if err := v.cmd.Start(); err != nil {
		log.Fatalf("ffmpeg: %v", err)
	}
	v.rd = bufio.NewReaderSize(pipe, v.w*v.h*4*2)
	v.next, v.eof = at, false
}

func (v *videoFeed) read(dst *image.RGBA) bool {
	if v.eof {
		return false
	}
	if _, err := io.ReadFull(v.rd, dst.Pix); err != nil {
		v.eof = true
		return false
	}
	v.next++
	return true
}

// frame garante que o bitmap mostra o frame idx da fonte (o último disponível se passar do fim).
func (v *videoFeed) frame(idx int) *ui.Bitmap {
	if idx == v.shown {
		return v.bm
	}
	if v.cmd == nil || idx < v.next || idx > v.next+90 {
		v.open(idx)
	}
	other := 1 - v.cur
	got := false
	for v.next <= idx {
		if !v.read(v.buf[other]) {
			break
		}
		got = true
	}
	if got {
		v.cur = other
		v.bm.Update(v.buf[v.cur]) // RGBA tomado sem cópia: o outro buffer recebe o próximo frame
		v.shown = idx
	}
	return v.bm
}

func (v *videoFeed) close() {
	if v.cmd != nil && v.cmd.Process != nil {
		_ = v.cmd.Process.Kill()
		_ = v.cmd.Wait()
	}
	v.cmd = nil
}
