import { Config } from "@remotion/cli/config";
// JPEG intermediário é mais rápido; PNG só se precisar de alpha.
Config.setVideoImageFormat("jpeg");
Config.setJpegQuality(92);
Config.setConcurrency(null); // usa ~metade dos núcleos; ajuste se a máquina sobrar
Config.setChromiumOpenGlRenderer("angle");
