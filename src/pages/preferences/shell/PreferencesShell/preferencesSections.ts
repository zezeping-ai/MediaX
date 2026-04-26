import type { Component } from "vue";
import AppearanceSection from "../../sections/appearance/index.vue";
import PlayerSection from "../../sections/player/index.vue";

export type SectionKey = "appearance" | "player";

export type PreferencesSection = {
  key: SectionKey;
  label: string;
  icon: string;
  component: Component;
};

export const PREFERENCES_SECTIONS: PreferencesSection[] = [
  {
    key: "appearance",
    label: "外观",
    icon: "mdi:palette-outline",
    component: AppearanceSection,
  },
  {
    key: "player",
    label: "播放器",
    icon: "mdi:play-circle-outline",
    component: PlayerSection,
  },
];
