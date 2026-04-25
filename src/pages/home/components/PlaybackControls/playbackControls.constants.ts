export const PREVIEW_SEEK_INTERVAL_MS = 100;

export const SPEED_OPTIONS = [0.5, 0.75, 1, 1.25, 1.5, 2] as const;
export const QUALITY_DOWNGRADE_LEVELS = [1080, 720, 480, 320] as const;

export const CIRCLE_BTN_BASE =
  "inline-flex h-10 min-h-10 w-10 min-w-10 items-center justify-center rounded-full p-0! leading-none transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/25 focus-visible:ring-offset-2 focus-visible:ring-offset-black/30 [&_.ant-btn-icon]:m-0! [&_.ant-btn-icon]:flex [&_.ant-btn-icon]:items-center [&_.ant-btn-icon]:justify-center disabled:opacity-55 disabled:cursor-not-allowed";

export const CIRCLE_BTN_GHOST =
  "border border-white/14 bg-white/8 text-white/90 shadow-[0_6px_16px_rgba(0,0,0,0.22)] hover:bg-white/12 hover:text-white";

export const CIRCLE_BTN_PRIMARY =
  "border-none bg-white text-slate-950 shadow-[0_10px_24px_rgba(0,0,0,0.35)] hover:bg-white";

export const PILL_BASE =
  "inline-flex items-center gap-2 rounded-full border border-white/10 bg-black/30 px-2.5 py-1.5 backdrop-blur-xl shadow-[0_10px_30px_rgba(0,0,0,0.35)]";

export const TINY_PILL_BTN =
  "inline-flex h-8 items-center gap-1 rounded-full border border-white/10 bg-white/5 px-3 text-xs font-semibold text-white/85 hover:bg-white/10 hover:text-white transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/25 focus-visible:ring-offset-2 focus-visible:ring-offset-black/30 [&_.ant-btn-icon]:m-0!";
