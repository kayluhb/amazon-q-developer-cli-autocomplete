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

// Types that run, don't insert text, or delete more than the query term
const NO_INLINE_TYPES = ["history", "auto-execute", "special", "shortcut"];
const CURSOR_PLACEHOLDER = "{cursor}";
// eslint-disable-next-line no-control-regex
const CONTROL_CHARS = /[\x00-\x1f\x7f]/;

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
    NO_INLINE_TYPES.includes(item.type ?? "")
  ) {
    return "";
  }

  let insertion = getFullInsertion(
    item,
    searchTerm,
    fuzzySearchEnabled,
    settings[SETTINGS.PREFER_VERBOSE_SUGGESTIONS] as boolean,
  );
  // The popup leaves the cursor at a trailing {cursor}, before any added space
  const endsAtCursor = insertion.endsWith(CURSOR_PLACEHOLDER);
  if (endsAtCursor) {
    insertion = insertion.slice(0, -CURSOR_PLACEHOLDER.length);
  }
  if (insertion.includes(CURSOR_PLACEHOLDER) || CONTROL_CHARS.test(insertion)) {
    return "";
  }

  const queryTerm = getQueryTermForSuggestion(item, searchTerm);
  if (!insertion.startsWith(queryTerm) || !buffer.endsWith(queryTerm)) {
    return "";
  }

  let suffix = insertion.slice(queryTerm.length);
  if (
    !endsAtCursor &&
    item.shouldAddSpace &&
    (settings[SETTINGS.INSERT_SPACE_AUTOMATICALLY] ?? true)
  ) {
    suffix += " ";
  }
  return suffix;
};

const SPEC_SOURCES = ["specs", "history-specs"];

let lastSent: { sessionId: string; buffer: string; suffix: string } | null =
  null;

// Tells tabterm the ghost text for the current buffer, computed from the top
// suggestion. Only sends when inline spec suggestions are on, the parse is for
// the current command, and something changed.
export const sendInlineSuggestion = (
  state: InlineSuffixState &
    Pick<AutocompleteState, "suggestions" | "command" | "parsedCommand">,
) => {
  const { figState, settings, suggestions, command, parsedCommand } = state;
  const { buffer, shellContext } = figState;
  const sessionId = shellContext?.sessionId;
  if (
    !sessionId ||
    !SPEC_SOURCES.includes(settings[SETTINGS.INLINE_SOURCE] as string) ||
    // Suggestions computed from an older parse would not match the new buffer
    parsedCommand !== command
  ) {
    return;
  }
  const suffix = inlineSuffix(suggestions[0], state);

  if (
    lastSent?.sessionId === sessionId &&
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
