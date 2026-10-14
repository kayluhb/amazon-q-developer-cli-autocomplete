import { SettingsChangedNotification, NotificationType } from "@tab/proto/fig";
import { _subscribeToCase, NotificationResponse } from "./notifications.js";

import {
  sendGetSettingsPropertyRequest,
  sendUpdateSettingsPropertyRequest,
} from "./requests.js";

export const didChange = {
  subscribe(
    handler: (
      notification: SettingsChangedNotification,
    ) => NotificationResponse | undefined,
  ) {
    return _subscribeToCase(
      NotificationType.NOTIFY_ON_SETTINGS_CHANGE,
      "settingsChangedNotification",
      handler,
    );
  },
};

export async function get(key: string) {
  return sendGetSettingsPropertyRequest({
    key,
  });
}

export async function set(key: string, value: unknown): Promise<void> {
  return sendUpdateSettingsPropertyRequest({
    key,
    value: JSON.stringify(value),
  });
}

export async function remove(key: string): Promise<void> {
  return sendUpdateSettingsPropertyRequest({
    key,
  });
}

export async function current() {
  const all = await sendGetSettingsPropertyRequest({});
  return JSON.parse(all.jsonBlob ?? "{}");
}
