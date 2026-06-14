export type LyricsSelectOption = {
  id: string;
  provider_id: string;
  title: string;
  artist: string;
  album?: string | null;
  duration_seconds?: number | null;
  preview: string;
  synced: boolean;
  lyrics_lrc?: string;
};
