export type MediaViewportEventMap = {
  ended: () => void;
  "quick-open-local": () => void;
  "quick-open-url": () => void;
};

export type PlaybackControlsEventMap = {
  mouseenter: () => void;
  mouseleave: () => void;
  mousemove: () => void;
  play: () => void;
  pause: (position: number) => void;
  stop: () => void;
  seek: (position: number) => void;
  "seek-preview": (position: number) => void;
  "change-rate": (value: number) => void;
  "change-volume": (value: number) => void;
  "change-quality": (value: string) => void;
  "overlay-interaction-change": (value: boolean) => void;
  "toggle-mute": () => void;
  "set-left-channel-volume": (value: number) => void;
  "set-right-channel-volume": (value: number) => void;
  "set-left-channel-muted": (value: boolean) => void;
  "set-right-channel-muted": (value: boolean) => void;
  "set-channel-routing": (value: string) => void;
  "toggle-cache": () => void;
  "toggle-lock": () => void;
  "export-audio": () => void;
};

export type UrlDialogEventMap = {
  confirm: () => void;
  cancel: () => void;
  clear: () => void;
  remove: (url: string) => void;
  select: (url: string) => void;
  play: (url: string) => void;
};
