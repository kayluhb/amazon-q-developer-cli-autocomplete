import {
  LocalStateChangedNotification,
  NotificationType,
} from "@tab/proto/fig";
import { _subscribeToCase, NotificationResponse } from "./notifications.js";

import {
  sendGetLocalStateRequest,
  sendUpdateLocalStateRequest,
} from "./requests.js";

export const didChange = {
  subscribe(
    handler: (
      notification: LocalStateChangedNotification,
    ) => NotificationResponse | undefined,
  ) {
    return _subscribeToCase(
      NotificationType.NOTIFY_ON_LOCAL_STATE_CHANGED,
      "localStateChangedNotification",
      handler,
    );
  },
};

export async function get(key: string) {
  const response = await sendGetLocalStateRequest({ key });
  return response.jsonBlob ? JSON.parse(response.jsonBlob) : null;
}

export async function set(key: string, value: unknown): Promise<void> {
  return sendUpdateLocalStateRequest({
    key,
    value: JSON.stringify(value),
  });
}

export async function remove(key: string) {
  return sendUpdateLocalStateRequest({
    key,
  });
}

export async function current() {
  const all = await sendGetLocalStateRequest({});
  return JSON.parse(all.jsonBlob ?? "{}");
}
