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
  ],
});
