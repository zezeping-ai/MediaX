/** 深色播放器共用表面色，控制条 / 歌词面板 / 频谱区保持一致 */
export const playerDarkSurfaceClass = {
  shell:
    "border-white/10 bg-[#121214] backdrop-blur-xl",
  panel:
    "border-white/10 bg-[#121214]",
  panelShadow:
    "shadow-[0_16px_40px_rgba(0,0,0,0.24)]",
  stereo:
    "border-white/8 bg-[#121214]",
  lyricsEdgeFade:
    "from-[#121214]",
} as const;
