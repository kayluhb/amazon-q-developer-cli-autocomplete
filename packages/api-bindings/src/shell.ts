import {
  NotificationType,
  ProcessChangedNotification,
  ShellPromptReturnedNotification,
  TextUpdate,
  HistoryUpdatedNotification,
  TextUpdateSchema,
} from "@tab/proto/fig";
import {
  sendInsertTextRequest,
  sendSetInlineSuggestionRequest,
} from "./requests.js";
import { _subscribeToCase, NotificationResponse } from "./notifications.js";
import { create } from "@bufbuild/protobuf";

export const processDidChange = {
  subscribe(
    handler: (
      notification: ProcessChangedNotification,
    ) => NotificationResponse | undefined,
  ) {
    return _subscribeToCase(
      NotificationType.NOTIFY_ON_PROCESS_CHANGED,
      "processChangeNotification",
      handler,
    );
  },
};

export const promptDidReturn = {
  subscribe(
    handler: (
      notification: ShellPromptReturnedNotification,
    ) => NotificationResponse | undefined,
  ) {
    return _subscribeToCase(
      NotificationType.NOTIFY_ON_PROMPT,
      "shellPromptReturnedNotification",
      handler,
    );
  },
};

export const historyUpdated = {
  subscribe(
    handler: (
      notification: HistoryUpdatedNotification,
    ) => NotificationResponse | undefined,
  ) {
    return _subscribeToCase(
      NotificationType.NOTIFY_ON_HISTORY_UPDATED,
      "historyUpdatedNotification",
      handler,
    );
  },
};

export async function insert(
  text: string,
  request?: Omit<TextUpdate, "insertion" | "$typeName">,
  terminalSessionId?: string,
) {
  if (request) {
    return sendInsertTextRequest({
      terminalSessionId,
      type: {
        case: "update",
        value: create(TextUpdateSchema, { ...request, insertion: text }),
      },
    });
  }
  return sendInsertTextRequest({
    terminalSessionId,
    type: { case: "text", value: text },
  });
}

export function setInlineSuggestion(
  buffer: string,
  suffix: string,
  terminalSessionId?: string,
) {
  return sendSetInlineSuggestionRequest({ buffer, suffix, terminalSessionId });
}
