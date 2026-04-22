import { createRouter, createWebHashHistory } from "vue-router";
import HomePage from "../pages/home/index.vue";
import PreferencesPage from "../pages/preferences/index.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: HomePage,
    },
    {
      path: "/preferences",
      name: "preferences",
      component: PreferencesPage,
    },
  ],
});
