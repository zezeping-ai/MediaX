import { invoke } from "@tauri-apps/api/core";

type InvokePayload = Record<string, unknown> | undefined;
type ResponseValidator<T> = (value: unknown) => value is T;

function nextRequestId(requestId?: string) {
  return requestId ?? crypto.randomUUID();
}

export function invokeMediaCommand<T>(command: string, payload?: InvokePayload) {
  return invoke<T>(command, payload);
}

export function invokeMediaCommandWithRequestId<T>(
  command: string,
  payload?: Record<string, unknown>,
  requestId?: string,
) {
  return invoke<T>(command, {
    ...(payload ?? {}),
    requestId: nextRequestId(requestId),
  });
}

export async function invokeMediaCommandValidated<T>(
  command: string,
  validator: ResponseValidator<T>,
  payload?: InvokePayload,
) {
  const response = await invoke<unknown>(command, payload);
  if (!validator(response)) {
    throw new Error(`Invalid response payload for command: ${command}`);
  }
  return response;
}

export async function invokeMediaCommandWithRequestIdValidated<T>(
  command: string,
  validator: ResponseValidator<T>,
  payload?: Record<string, unknown>,
  requestId?: string,
) {
  const response = await invoke<unknown>(command, {
    ...(payload ?? {}),
    requestId: nextRequestId(requestId),
  });
  if (!validator(response)) {
    throw new Error(`Invalid response payload for command: ${command}`);
  }
  return response;
}
