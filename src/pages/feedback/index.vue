<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { message } from "ant-design-vue";
import { invokeMediaCommand } from "@/modules/media-command";
import { usePreferences } from "@/modules/preferences";

interface FeedbackFormState {
  issue: string;
  description: string;
  contact: string;
}

const form = reactive<FeedbackFormState>({
  issue: "",
  description: "",
  contact: "",
});
const submitting = ref(false);
const { resolvedTheme } = usePreferences();
const isDark = computed(() => resolvedTheme.value === "dark");

async function submitFeedback() {
  const issue = form.issue.trim();
  const description = form.description.trim();
  const contact = form.contact.trim();
  if (!issue || !description) {
    message.warning("请填写问题和描述");
    return;
  }
  submitting.value = true;
  try {
    await invokeMediaCommand("submit_user_feedback", {
      payload: {
        issue,
        description,
        contact,
      },
    });
    message.success("反馈已发送，感谢你的建议");
    form.issue = "";
    form.description = "";
    form.contact = "";
  } catch (error) {
    const errorText = error instanceof Error ? error.message : String(error);
    message.error(`发送失败：${errorText}`);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <main
    class="h-screen w-screen p-4"
    :class="isDark ? 'bg-[#111826]' : 'bg-[#f5f7fa]'"
  >
    <section
      class="mx-auto w-full max-w-[760px] rounded-xl p-4 shadow-sm md:p-5"
      :class="isDark ? 'bg-[#0f1724] border border-white/10' : 'bg-white'"
    >
      <header class="mb-4">
        <h1
          class="text-base font-semibold"
          :class="isDark ? 'text-white/90' : 'text-[rgba(0,0,0,0.88)]'"
        >
          用户反馈
        </h1>
        <p
          class="mt-1 text-xs"
          :class="isDark ? 'text-white/55' : 'text-[rgba(0,0,0,0.45)]'"
        >
          提交问题、描述和联系方式，我们会尽快跟进。
        </p>
      </header>
      <a-form layout="vertical">
        <a-form-item label="问题" required>
          <a-input
            v-model:value="form.issue"
            :maxlength="80"
            :disabled="submitting"
            placeholder="例如：更新后无法启动"
          />
        </a-form-item>
        <a-form-item label="描述" required>
          <textarea
            v-model="form.description"
            class="w-full rounded-md px-3 py-2 text-[13px] leading-5 outline-none transition-colors duration-150"
            :class="isDark
              ? 'border border-white/15 bg-[#0b1220] text-white/90 placeholder:text-white/35 focus:border-[#3b82f6]'
              : 'border border-[#d9d9d9] bg-white text-[rgba(0,0,0,0.88)] focus:border-[#1677ff]'"
            rows="8"
            maxlength="1000"
            :disabled="submitting"
            placeholder="请尽量描述复现步骤、预期行为和实际行为"
          />
        </a-form-item>
        <a-form-item label="联系方式（选填）">
          <a-input
            v-model:value="form.contact"
            :maxlength="120"
            :disabled="submitting"
            placeholder="微信 / 邮箱 / 手机号"
          />
        </a-form-item>
      </a-form>
      <footer class="flex justify-end">
        <a-button type="primary" :loading="submitting" @click="submitFeedback">
          提交反馈
        </a-button>
      </footer>
    </section>
  </main>
</template>
