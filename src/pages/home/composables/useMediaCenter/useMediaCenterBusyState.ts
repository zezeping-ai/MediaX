import { ref } from "vue";

export function useMediaCenterBusyState(toUserErrorMessage: (error: unknown) => string) {
  const isBusy = ref(false);
  const errorMessage = ref("");

  async function withBusyState(action: () => Promise<void>) {
    isBusy.value = true;
    try {
      await action();
    } catch (error) {
      errorMessage.value = toUserErrorMessage(error);
    } finally {
      isBusy.value = false;
    }
  }

  return {
    errorMessage,
    isBusy,
    withBusyState,
  };
}
