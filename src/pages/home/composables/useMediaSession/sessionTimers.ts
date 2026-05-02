interface SessionTimersOptions {
  markTelemetryStaleIfNeeded: () => void;
}

export function startSessionTimers(options: SessionTimersOptions) {
  const telemetryStaleTimer = window.setInterval(() => {
    options.markTelemetryStaleIfNeeded();
  }, 500);

  return {
    dispose() {
      window.clearInterval(telemetryStaleTimer);
    },
  };
}
