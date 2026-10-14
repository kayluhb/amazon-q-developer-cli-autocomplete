import {
  Action,
  ActionListSchema,
  KeybindingPressedNotification,
  NotificationType,
} from "@tab/proto/fig";
import { sendUpdateApplicationPropertiesRequest } from "./requests.js";
import { _subscribeToCase, NotificationResponse } from "./notifications.js";
import { create } from "@bufbuild/protobuf";

export function pressed(
  handler: (
    notification: KeybindingPressedNotification,
  ) => NotificationResponse | undefined,
) {
  return _subscribeToCase(
    NotificationType.NOTIFY_ON_KEYBINDING_PRESSED,
    "keybindingPressedNotification",
    handler,
  );
}

export function setInterceptKeystrokes(
  actions: Omit<Action, "$typeName">[],
  intercept: boolean,
  globalIntercept?: boolean,
  currentTerminalSessionId?: string,
) {
  sendUpdateApplicationPropertiesRequest({
    interceptBoundKeystrokes: intercept,
    interceptGlobalKeystrokes: globalIntercept,
    actionList: create(ActionListSchema, { actions }),
    currentTerminalSessionId,
  });
}
