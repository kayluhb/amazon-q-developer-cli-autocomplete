import { NotificationType } from "@tab/proto/fig";
import { _subscribeToCase, NotificationResponse } from "./notifications.js";

export function subscribe<T>(
  eventName: string,
  handler: (payload: T) => NotificationResponse | undefined,
) {
  return _subscribeToCase(
    NotificationType.NOTIFY_ON_EVENT,
    "eventNotification",
    ({ eventName: name, payload }) => {
      if (name === eventName) {
        try {
          return handler(payload ? JSON.parse(payload) : null);
        } catch (_err) {
          // ignore on json parse failure (invalid event).
        }
      }
      return { unsubscribe: false };
    },
  );
}
