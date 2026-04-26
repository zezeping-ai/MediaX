interface SessionTimersOptions {
  getSnapshot: () => Promise<void>;
  markTelemetryStaleIfNeeded: () => void;
}

export function startSessionTimers(options: SessionTimersOptions) {
  const snapshotPollingTimer = window.setInterval(() => {
    void options.getSnapshot();
  }, 1000);
  const telemetryStaleTimer = window.setInterval(() => {
    options.markTelemetryStaleIfNeeded();
  }, 500);

  return {
    dispose() {
      window.clearInterval(snapshotPollingTimer);
      window.clearInterval(telemetryStaleTimer);
    },
  };
}
