import { createRouter, createWebHashHistory } from "vue-router";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: () => import("@/pages/home/index.vue"),
    },
    {
      path: "/preferences",
      name: "preferences",
      component: () => import("@/pages/preferences/index.vue"),
    },
    {
      path: "/tools/video-transcode",
      name: "video-transcode",
      component: () => import("@/pages/video-transcode/index.vue"),
    },
    {
      path: "/tools/audio-transcode",
      name: "audio-transcode",
      component: () => import("@/pages/audio-transcode/index.vue"),
    },
    {
      path: "/tools/image-compress",
      name: "image-compress",
      component: () => import("@/pages/image-compress/index.vue"),
    },
    {
      path: "/feedback",
      name: "feedback",
      component: () => import("@/pages/feedback/index.vue"),
    },
  ],
});
