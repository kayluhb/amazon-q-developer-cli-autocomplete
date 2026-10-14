import logger from "loglevel";
import { Shell } from "@tab/api-bindings";
import { SETTINGS } from "@tab/api-bindings-wrappers";
import { Suggestion } from "@tab/shared/internal";
import { getQueryTermForSuggestion } from "../suggestions/helpers";
import { getFullInsertion } from "./insertion";
import { AutocompleteState, Visibility } from "./types";

type InlineSuffixState = Pick<
  AutocompleteState,
  | "parserResult"
  | "figState"
  | "visibleState"
  | "fuzzySearchEnabled"
  | "settings"
>;

const NON_INSERTING_TYPES = ["history", "auto-execute", "special"];

// The text the popup would add after the buffer if `item` were inserted, or
// "" when that can't be shown as ghost text after the cursor.
export const inlineSuffix = (
  item: Suggestion | undefined,
  state: InlineSuffixState,
): string => {
  const {
    parserResult: { searchTerm },
    figState: { buffer, cursorLocation },
    visibleState,
    fuzzySearchEnabled,
    settings,
  } = state;

  if (
    !item ||
    cursorLocation !== buffer.length ||
    visibleState === Visibility.HIDDEN_UNTIL_KEYPRESS ||
    NON_INSERTING_TYPES.includes(item.type ?? "")
  ) {
    return "";
  }

  let insertion = getFullInsertion(
    item,
    searchTerm,
    fuzzySearchEnabled,
    settings[SETTINGS.PREFER_VERBOSE_SUGGESTIONS] as boolean,
  );
  if (insertion.endsWith("{cursor}")) {
    insertion = insertion.slice(0, -"{cursor}".length);
  }
  if (insertion.includes("{cursor}") || insertion.includes("\n")) {
    return "";
  }

  const queryTerm = getQueryTermForSuggestion(item, searchTerm);
  if (!insertion.startsWith(queryTerm) || !buffer.endsWith(queryTerm)) {
    return "";
  }

  let suffix = insertion.slice(queryTerm.length);
  if (
    item.shouldAddSpace &&
    (settings[SETTINGS.INSERT_SPACE_AUTOMATICALLY] ?? true)
  ) {
    suffix += " ";
  }
  return suffix;
};

let lastSent: { sessionId: string; buffer: string; suffix: string } | null =
  null;

const SPEC_SOURCES = ["specs", "history-specs"];

// Tells tabterm the ghost text for the current buffer, computed from the top
// suggestion. Only sends when inline spec suggestions are on and something
// changed.
export const sendInlineSuggestion = (
  state: InlineSuffixState & Pick<AutocompleteState, "suggestions">,
) => {
  const sessionId = state.figState.shellContext?.sessionId;
  if (
    !sessionId ||
    !SPEC_SOURCES.includes(state.settings[SETTINGS.INLINE_SOURCE] as string)
  ) {
    return;
  }
  const { buffer } = state.figState;
  const suffix = inlineSuffix(state.suggestions[0], state);

  if (
    lastSent &&
    lastSent.sessionId === sessionId &&
    lastSent.buffer === buffer &&
    lastSent.suffix === suffix
  ) {
    return;
  }
  lastSent = { sessionId, buffer, suffix };

  Shell.setInlineSuggestion(buffer, suffix, sessionId).catch((err) => {
    logger.debug("Failed to send inline suggestion", err);
  });
};
