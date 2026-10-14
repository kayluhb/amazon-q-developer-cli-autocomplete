import {
  EditBufferChangedNotification,
  NotificationType,
} from "@tab/proto/fig";
import { _subscribeToCase, NotificationResponse } from "./notifications.js";

export function subscribe(
  handler: (
    notification: EditBufferChangedNotification,
  ) => NotificationResponse | undefined,
) {
  return _subscribeToCase(
    NotificationType.NOTIFY_ON_EDITBUFFFER_CHANGE,
    "editBufferNotification",
    handler,
  );
}
