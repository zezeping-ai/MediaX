import { ref } from "vue";

export function useMediaCenterBusyState(toUserErrorMessage: (error: unknown) => string) {
  const isBusy = ref(false);
  const errorMessage = ref("");

  function captureError(error: unknown) {
    errorMessage.value = toUserErrorMessage(error);
  }

  async function withBusyState(action: () => Promise<void>) {
    isBusy.value = true;
    try {
      await action();
    } catch (error) {
      captureError(error);
    } finally {
      isBusy.value = false;
    }
  }

  return {
    captureError,
    errorMessage,
    isBusy,
    withBusyState,
  };
}
