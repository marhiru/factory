mod agenda;
mod align;
mod analyze;
mod auth;
mod init;
mod kv;
mod net;
mod platform;
mod decode;
mod export;
mod formats;
mod mix;
mod frames;
mod probe;
mod qa;
mod sheet;
mod tempo;
mod transcribe;
mod util;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "vf", version, about = "video-factory: análise frame a frame, transcrição, QA e export de vídeo")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Clone)]
struct DecodeArgs {
    /// largura de análise (px); o decode já sai reduzido
    #[arg(long, default_value_t = 256)]
    width: usize,
    /// processos ffmpeg em paralelo (segmentos). O decode por software já usa todos os núcleos;
    /// 2 ajuda em codecs com threading fraco
    #[arg(short, long, default_value_t = 2)]
    jobs: usize,
    /// decodificação por hardware (VideoToolbox). No Apple Silicon costuma ser MAIS LENTA para análise
    /// porque cada frame precisa voltar da GPU; use só para codecs pesados (ProRes/HEVC 10-bit 4K)
    #[arg(long)]
    hw: bool,
    /// limiar de corte (distância de histograma 0-1)
    #[arg(long, default_value_t = 0.40)]
    threshold: f32,
    /// duração mínima de plano em frames
    #[arg(long, default_value_t = 6)]
    min_shot: usize,
}

impl DecodeArgs {
    fn opts(&self, thumb_every: usize) -> frames::FramesOpts {
        frames::FramesOpts {
            width: self.width,
            jobs: self.jobs.max(1),
            hwaccel: self.hw,
            threshold: self.threshold,
            min_shot: self.min_shot,
            thumb_every,
        }
    }
}

#[derive(Args, Clone)]
struct WhisperArgs {
    /// modelo whisper.cpp (nome ou caminho .bin); baixado em ~/.cache/vf/models
    #[arg(long, default_value = "large-v3-turbo-q5_0")]
    model: String,
    /// idioma (pt, en, auto...)
    #[arg(long, default_value = "auto")]
    lang: String,
    /// prompt inicial (nomes próprios, grafias)
    #[arg(long)]
    prompt: Option<String>,
    /// timestamps por token com DTW (mais precisos, um pouco mais lento)
    #[arg(long)]
    dtw: bool,
    #[arg(long, default_value_t = 8)]
    threads: usize,
}

impl WhisperArgs {
    fn opts(&self) -> transcribe::TranscribeOpts {
        transcribe::TranscribeOpts { model: self.model.clone(), lang: self.lang.clone(), threads: self.threads, prompt: self.prompt.clone(), dtw: self.dtw }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Specs do arquivo em JSON
    Probe { input: PathBuf },
    /// Estatística por frame + cortes -> frames.jsonl, shots.json, probe.json
    Frames {
        input: PathBuf,
        #[arg(short, long, default_value = "vf_out")]
        out: PathBuf,
        #[command(flatten)]
        d: DecodeArgs,
    },
    /// Keyframe por plano (ou a cada N s) + contact sheets
    Keyframes {
        input: PathBuf,
        #[arg(short, long, default_value = "vf_out")]
        out: PathBuf,
        /// usar este shots.json em vez de detectar
        #[arg(long)]
        shots: Option<PathBuf>,
        /// um frame a cada N segundos em vez de um por plano
        #[arg(long)]
        every: Option<f64>,
        #[arg(long, default_value_t = 1280)]
        side: u32,
        #[command(flatten)]
        d: DecodeArgs,
    },
    /// Transcrição com tempo por palavra -> words.json, transcript.srt, transcript.txt
    Transcribe {
        input: PathBuf,
        #[arg(short, long, default_value = ".")]
        out: PathBuf,
        /// converter JSON do Speech-to-Text do ElevenLabs em vez de rodar o whisper
        #[arg(long)]
        elevenlabs: bool,
        #[command(flatten)]
        w: WhisperArgs,
    },
    /// Estudo completo de um vídeo (arquivo ou URL): cortes, keyframes, sheets, transcrição, ritmo
    Analyze {
        input: String,
        #[arg(short, long, default_value = "vf_analysis")]
        out: PathBuf,
        #[arg(long)]
        no_transcribe: bool,
        #[arg(long, default_value_t = 1280)]
        side: u32,
        #[command(flatten)]
        d: DecodeArgs,
        #[command(flatten)]
        w: WhisperArgs,
    },
    /// QA do vídeo final contra o formato de entrega
    Qa {
        input: PathBuf,
        /// yt-long | yt-short | tiktok
        #[arg(short, long)]
        formato: String,
        #[arg(short, long, default_value = "qa")]
        out: PathBuf,
        /// shots.json/plan.json com flash/xfade/planned para ignorar transições planejadas
        #[arg(long)]
        plan: Option<PathBuf>,
        /// hold mínimo (frames) para acusar congelamento (padrão: qa.freeze_frames do formato)
        #[arg(long)]
        freeze: Option<usize>,
        /// miniatura a cada N frames nas sheets
        #[arg(long, default_value_t = 10)]
        sheet_every: usize,
        /// rascunho: WARN não reprova. O padrão é ESTRITO (WARN = FAIL, sai com código 1)
        #[arg(long)]
        lenient: bool,
        /// aceito por compatibilidade (estrito já é o padrão)
        #[arg(long, hide = true)]
        strict: bool,
        /// factory.yaml com `qa_excecoes: [{check, motivo}]` (padrão: ./factory.yaml se existir)
        #[arg(long)]
        factory: Option<PathBuf>,
        /// trilha de voz isolada (ex.: public/voice/voice.wav) para checar buracos na narração
        #[arg(long)]
        voice: Option<PathBuf>,
        #[arg(long)]
        formats: Option<PathBuf>,
        #[command(flatten)]
        d: DecodeArgs,
    },
    /// Export no preset do formato (enquadramento, encode, loudnorm, mux)
    Export {
        input: PathBuf,
        #[arg(short, long)]
        formato: String,
        #[arg(short, long)]
        out: PathBuf,
        /// trilha final (ex.: audio/out/mix.wav) para mux
        #[arg(long)]
        audio: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "pad")]
        fit: export::Fit,
        /// encoder por hardware (VideoToolbox), bem mais rápido, qualidade um pouco menor
        #[arg(long)]
        fast: bool,
        #[arg(long)]
        no_loudnorm: bool,
        #[arg(long)]
        reencode: bool,
        /// tamanho explícito WxH
        #[arg(long)]
        size: Option<String>,
        #[arg(long)]
        formats: Option<PathBuf>,
    },
    /// Amarra os beats do roteiro às palavras da narração (adiciona w0, w1, start, end)
    Align {
        /// roteiro/beats.json
        beats: PathBuf,
        /// words.json (vf transcribe)
        words: PathBuf,
        /// saída (padrão: sobrescreve beats.json)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Mixa as faixas de áudio do edl.json (voz, música com ducking, sfx) num WAV 48 kHz
    Mix {
        /// data/edl.json
        edl: PathBuf,
        #[arg(short, long, default_value = "public/audio/mix.wav")]
        out: PathBuf,
        /// pasta base dos `src` (padrão: public)
        #[arg(long, default_value = "public")]
        root: PathBuf,
        /// redução aproximada da música sob a voz (dB)
        #[arg(long, default_value_t = 10.0)]
        duck: f64,
    },
    /// Folha de contato de uma pasta de imagens (curadoria de lotes do Google Flow/IA) + sheet.json com tamanhos
    Sheet {
        dir: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 6)]
        cols: usize,
        #[arg(long, default_value_t = 4)]
        rows: usize,
        /// largura de cada miniatura (px)
        #[arg(long, default_value_t = 400)]
        side: u32,
        /// largura mínima para o zoom do vídeo (px); abaixo disso a imagem é marcada como "upscale"
        #[arg(long, default_value_t = 2150)]
        min_w: u32,
    },
    /// Lista os formatos de entrega
    Formats {
        #[arg(long)]
        formats: Option<PathBuf>,
    },
    /// Inicializa a pasta atual como projeto da fábrica: vf init type="short,long" platform="yt,tiktok"
    /// [motor=... idioma=pt duracao=60 canal=<pasta> dir=<pasta> --install --offline]
    Init {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "CHAVE=VALOR")]
        args: Vec<String>,
    },
    /// Contas nas plataformas (OAuth, várias por plataforma): vf auth platform=yt [account=<apelido>] [client=<json baixado>] | vf auth status | vf auth use platform=yt account=<apelido> | vf auth logout platform=yt account=<apelido>
    Auth {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "CHAVE=VALOR")]
        args: Vec<String>,
    },
    /// Envia/posta vídeo longo ou Short: vf publish <mp4> platform=yt,tiktok [mode=rascunho|privado|nao-listado|amigos|publico] [at="2026-11-20 18:00"|+2h] [meta=out/publicacao.json thumb=capa.jpg account=a,b]
    Publish {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "ARQUIVO CHAVE=VALOR")]
        args: Vec<String>,
    },
    /// Corrige título/descrição/tags de vídeo já enviado (YouTube): vf meta <ID> platform=yt meta=<json> [aplicar=1]
    Meta {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "ID CHAVE=VALOR")]
        args: Vec<String>,
    },
    /// Posts agendados localmente (TikTok): vf agenda [list] | run | cancel id=<id> | retry id=<id> | install | uninstall
    Agenda {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "AÇÃO CHAVE=VALOR")]
        args: Vec<String>,
    },
    /// Analíticos: vf analytics platform=yt [account=<apelido>|all] [video=ID1,ID2] [dias=28] [max=50] [comentarios=1] [curva=1] -> out/analytics/<p>-<conta>-<data>.json
    Analytics {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "CHAVE=VALOR")]
        args: Vec<String>,
    },
    /// Verifica dependências externas
    Doctor,
}

fn sheet_dir(dir: &std::path::Path, out: Option<PathBuf>, cols: usize, rows: usize, side: u32, min_w: u32) -> Result<()> {
    use rayon::prelude::*;
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp")))
        .collect();
    files.sort();
    anyhow::ensure!(!files.is_empty(), "nenhuma imagem em {}", dir.display());
    let loaded: Vec<(PathBuf, image::RgbImage)> = files
        .par_iter()
        .filter_map(|p| match image::open(p) {
            Ok(im) => Some((p.clone(), im.to_rgb8())),
            Err(e) => {
                eprintln!("ignorado {}: {e}", p.display());
                None
            }
        })
        .collect();
    let out = out.unwrap_or_else(|| dir.join("_sheet"));
    util::ensure_dir(&out)?;
    let mut info = Vec::new();
    let tiles: Vec<sheet::Tile> = loaded
        .into_iter()
        .enumerate()
        .map(|(k, (p, im))| {
            let (w, h) = (im.width(), im.height());
            let up = w < min_w;
            info.push(serde_json::json!({"n": k + 1, "file": p.file_name().unwrap().to_string_lossy(), "w": w, "h": h, "upscale": up}));
            let thumb = image::imageops::thumbnail(&im, side * 2, (side as u64 * 2 * h as u64 / w.max(1) as u64) as u32);
            sheet::Tile { img: thumb, label: format!("{}{}", k + 1, if up { " UP" } else { "" }) }
        })
        .collect();
    let n = tiles.len();
    let sheets = sheet::contact_sheets(tiles, cols, rows, side, &out.join("sheet_"))?;
    util::write_json(&out.join("sheet.json"), &info)?;
    let ups = info.iter().filter(|i| i["upscale"].as_bool() == Some(true)).count();
    eprintln!("{n} imagens -> {} folha(s) em {} ({ups} abaixo de {min_w}px: marcadas UP)", sheets.len(), out.display());
    for s in sheets {
        println!("{}", s.display());
    }
    Ok(())
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("erro: {e:#}");
        std::process::exit(2);
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Probe { input } => println!("{}", serde_json::to_string_pretty(&probe::probe(&input)?)?),
        Cmd::Frames { input, out, d } => {
            util::ensure_dir(&out)?;
            let fa = frames::analyze(&input, &d.opts(0))?;
            frames::write_outputs(&fa, &out)?;
            let fps = fa.frames.len() as f64 / fa.decode_secs.max(1e-9);
            eprintln!(
                "{} frames, {} planos em {:.2} s ({:.0} fps de análise, {}x{}) -> {}",
                fa.frames.len(),
                fa.shots.len(),
                fa.decode_secs,
                fps,
                fa.opts.width,
                fa.opts.height,
                out.display()
            );
        }
        Cmd::Keyframes { input, out, shots, every, side, d } => {
            util::ensure_dir(&out)?;
            let p = probe::probe(&input)?;
            let (times, labels): (Vec<f64>, Vec<String>) = if let Some(step) = every {
                let n = (p.duration / step).floor() as usize + 1;
                (0..n).map(|k| k as f64 * step).filter(|&t| t < p.duration).map(|t| (t, util::tc(t))).unzip()
            } else {
                let list: Vec<frames::Shot> = match shots {
                    Some(sp) => {
                        let j: serde_json::Value = serde_json::from_slice(&std::fs::read(&sp)?)?;
                        serde_json::from_value(j.get("shots").cloned().unwrap_or(j)).context("shots.json inválido")?
                    }
                    None => {
                        let fa = frames::analyze(&input, &d.opts(0))?;
                        frames::write_outputs(&fa, &out)?;
                        fa.shots
                    }
                };
                list.iter().map(|s| ((s.ta + s.tb) / 2.0, format!("#{} {} {:.1}s", s.id, util::tc(s.ta), s.dur))).unzip()
            };
            let kdir = out.join("keyframes");
            util::ensure_dir(&kdir)?;
            let kfs = sheet::extract_keyframes(&input, &times, &kdir, "k_", side, d.hw)?;
            let tiles = kfs
                .iter()
                .zip(labels)
                .filter_map(|(k, label)| Some(sheet::Tile { img: image::open(k).ok()?.to_rgb8(), label }))
                .collect();
            let vertical = p.height > p.width;
            let sdir = out.join("sheets");
            util::ensure_dir(&sdir)?;
            let sh = sheet::contact_sheets(tiles, if vertical { 6 } else { 4 }, if vertical { 2 } else { 5 }, if vertical { 220 } else { 320 }, &sdir.join("sheet_"))?;
            eprintln!("{} keyframes, {} sheets em {}", kfs.len(), sh.len(), out.display());
        }
        Cmd::Transcribe { input, out, elevenlabs, w } => {
            util::ensure_dir(&out)?;
            let t0 = std::time::Instant::now();
            let words = if elevenlabs {
                transcribe::from_elevenlabs(&serde_json::from_slice(&std::fs::read(&input)?)?)
            } else {
                transcribe::transcribe(&input, &out, &w.opts())?
            };
            transcribe::write_words(&words, &out)?;
            eprintln!("{} palavras em {:.1} s -> {}", words.len(), t0.elapsed().as_secs_f64(), out.join("words.json").display());
        }
        Cmd::Analyze { input, out, no_transcribe, side, d, w } => {
            let tr = (!no_transcribe).then(|| w.opts());
            analyze::analyze(&input, &out, &analyze::AnalyzeOpts { frames: d.opts(0), transcribe: tr, keyframe_side: side })?;
        }
        Cmd::Qa { input, formato, out, plan, freeze, sheet_every, lenient, strict: _, factory, voice, formats: fp, d } => {
            util::ensure_dir(&out)?;
            let fmt = formats::get(fp.as_deref(), &formato)?;
            let plan_v = match plan {
                Some(p) => Some(serde_json::from_slice(&std::fs::read(&p)?)?),
                None => None,
            };
            let fa = frames::analyze(&input, &d.opts(sheet_every))?;
            frames::write_outputs(&fa, &out)?;
            let factory = factory.or_else(|| Some(PathBuf::from("factory.yaml")).filter(|p| p.exists()));
            let exceptions = factory.as_deref().map(qa::load_exceptions).unwrap_or_default();
            let o = qa::QaOpts { plan: plan_v.as_ref(), freeze_frames: freeze, strict: !lenient, exceptions, voice: voice.as_deref() };
            let worst = qa::run_qa(&fa, &formato, &fmt, &o, &out)?;
            if worst == qa::Level::FAIL {
                eprintln!("QA REPROVADO ({}): corrija ou registre a exceção com motivo em factory.yaml > qa_excecoes", if lenient { "leniente" } else { "estrito" });
                std::process::exit(1);
            }
        }
        Cmd::Export { input, formato, out, audio, fit, fast, no_loudnorm, reencode, size, formats: fp } => {
            let fmt = formats::get(fp.as_deref(), &formato)?;
            let size = match size {
                Some(s) => {
                    let (a, b) = s.split_once('x').context("--size use WxH")?;
                    Some((a.parse()?, b.parse()?))
                }
                None => None,
            };
            export::export(&input, &out, &fmt, &export::ExportOpts { audio, fit, fast, loudnorm: !no_loudnorm, reencode, size })?;
        }
        Cmd::Align { beats, words, out } => {
            let mut b: serde_json::Value = serde_json::from_slice(&std::fs::read(&beats)?).context("beats.json")?;
            let w: Vec<transcribe::Word> = serde_json::from_slice(&std::fs::read(&words)?).context("words.json")?;
            let spans = align::align(&mut b, &w)?;
            let mut bad = 0;
            for s in &spans {
                let t = |k: Option<usize>| k.map(|k| util::tc(w[k].start)).unwrap_or("--".into());
                let ok = s.total == 0 || s.matched * 2 >= s.total;
                if !ok {
                    bad += 1;
                }
                println!("beat {:>4}  palavras {:>4}-{:<4}  {} -> {}  casadas {}/{}{}", s.id, s.w0.map_or("-".into(), |x| x.to_string()), s.w1.map_or("-".into(), |x| x.to_string()), t(s.w0), s.w1.map(|k| util::tc(w[k].end)).unwrap_or("--".into()), s.matched, s.total, if ok { "" } else { "  <- CONFERIR" });
            }
            util::write_json(out.as_deref().unwrap_or(&beats), &b)?;
            if bad > 0 {
                eprintln!("{bad} beat(s) com menos da metade das palavras casadas: a narração mudou o texto?");
            }
        }
        Cmd::Mix { edl, out, root, duck } => {
            if let Some(d) = out.parent() {
                util::ensure_dir(d)?;
            }
            mix::mix(&edl, &root, &out, &mix::MixOpts { duck_db: duck })?;
        }
        Cmd::Formats { formats: fp } => {
            for (k, f) in formats::load(fp.as_deref())? {
                println!(
                    "{k:9} {:22} {}x{} {}  {}-{} s (alvo {})  {} LUFS  {} wpm",
                    f.label, f.width, f.height, f.aspect, f.duration.min, f.duration.max, f.duration.target, f.lufs, f.wpm
                );
            }
        }
        Cmd::Sheet { dir, out, cols, rows, side, min_w } => sheet_dir(&dir, out, cols, rows, side, min_w)?,
        Cmd::Init { args } => init::run(&args)?,
        Cmd::Auth { args } => auth::run(&args)?,
        Cmd::Publish { args } => platform::publish(&args)?,
        Cmd::Meta { args } => platform::meta(&args)?,
        Cmd::Agenda { args } => agenda::run(&args)?,
        Cmd::Analytics { args } => platform::analytics(&args)?,
        Cmd::Doctor => doctor(),
    }
    Ok(())
}

fn doctor() {
    let items = [
        ("ffmpeg", util::ffmpeg(), "obrigatório (brew install ffmpeg)"),
        ("ffprobe", util::ffprobe(), "obrigatório"),
        ("whisper-cli", std::env::var("VF_WHISPER").unwrap_or("whisper-cli".into()), "transcrição (brew install whisper-cpp)"),
        ("yt-dlp", "yt-dlp".into(), "analyze de URL (brew install yt-dlp)"),
        ("bun", "bun".into(), "scripts JS/TS, dependências e render (bun.sh)"),
        ("curl", "curl".into(), "vf auth / publish / analytics"),
        ("git", "git".into(), "vf init (bibliotecas de FX em ref/fx)"),
    ];
    for (label, bin, why) in items {
        match util::which(&bin) {
            Some(p) => println!("ok    {label:12} {}", p.display()),
            None => println!("FALTA {label:12} {why}"),
        }
    }
    let models = Path::new(&std::env::var("HOME").unwrap_or_default()).join(".cache/vf/models");
    let n = std::fs::read_dir(&models).map(|d| d.count()).unwrap_or(0);
    println!("modelos whisper em {} : {n}", models.display());
    println!("núcleos: {}", util::jobs(None));
}
