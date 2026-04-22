<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import { useMouse } from "@vueuse/core";
import { capitalize, trim } from "lodash-es";
import { invoke } from "@tauri-apps/api/core";
import DemoTsx from "../../components/DemoTsx";
import { useCounterStore } from "../../stores/counter";

const greetMsg = ref("");
const name = ref("");
const counter = useCounterStore();
const { x, y } = useMouse();

const displayName = computed(() => capitalize(trim(name.value || "guest")));

async function greet() {
  greetMsg.value = await invoke<string>("greet", { name: trim(name.value) });
  counter.increment();
}

watch(name, () => {
  greetMsg.value = "";
});
</script>

<template>
  <main class="container">
    <a-space direction="vertical" size="large" style="width: 100%">
      <a-typography-title :level="1">Welcome to Tauri + Vue</a-typography-title>

      <a-typography-text type="secondary">
        Pinia 计数：{{ counter.count }}（双倍 {{ counter.doubled }}） · 鼠标
        {{ Math.round(x) }}, {{ Math.round(y) }}（@vueuse/core）
      </a-typography-text>

      <div class="row">
        <a href="https://vite.dev" target="_blank" rel="noreferrer">
          <img src="/vite.svg" class="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank" rel="noreferrer">
          <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://vuejs.org/" target="_blank" rel="noreferrer">
          <img src="../../assets/vue.svg" class="logo vue" alt="Vue logo" />
        </a>
      </div>

      <a-typography-paragraph>
        预览名（lodash-es <code>trim</code> + <code>capitalize</code>）：
        <strong>{{ displayName }}</strong>
        <Icon icon="mdi:hand-wave" class="wave-icon" aria-hidden="true" />
      </a-typography-paragraph>

      <a-form layout="inline" @finish.prevent="greet">
        <a-form-item>
          <a-input
            id="greet-input"
            v-model:value="name"
            placeholder="Enter a name..."
            allow-clear
            style="min-width: 220px"
          />
        </a-form-item>
        <a-form-item>
          <a-button type="primary" html-type="submit">Greet</a-button>
        </a-form-item>
      </a-form>

      <a-typography-paragraph v-if="greetMsg">{{ greetMsg }}</a-typography-paragraph>

      <DemoTsx />
    </a-space>
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.row {
  display: flex;
  justify-content: center;
}

.wave-icon {
  display: inline-block;
  vertical-align: -0.15em;
  margin-left: 6px;
  font-size: 1.25em;
}
</style>

