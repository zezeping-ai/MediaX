import { defineComponent } from "vue";

/** 用于验证 `@vitejs/plugin-vue-jsx` 是否生效 */
export default defineComponent({
  name: "DemoTsx",
  setup() {
    return () => (
      <p class="demo-tsx">
        本段由 <code>@vitejs/plugin-vue-jsx</code> 渲染。
      </p>
    );
  },
});
