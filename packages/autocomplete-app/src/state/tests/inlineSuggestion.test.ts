import { beforeEach, describe, expect, it, vi } from "vitest";
import { Shell } from "@tab/api-bindings";
import { SETTINGS } from "@tab/api-bindings-wrappers";
import { Suggestion } from "@tab/shared/internal";
import { inlineSuffix, sendInlineSuggestion } from "../inlineSuggestion";
import { Visibility } from "../types";

vi.mock("@tab/api-bindings", () => ({
  Shell: { setInlineSuggestion: vi.fn(() => Promise.resolve()) },
}));

const makeState = (
  buffer: string,
  searchTerm: string,
  {
    cursorLocation = buffer.length,
    visibleState = Visibility.VISIBLE,
    insertSpace = true,
  } = {},
) =>
  ({
    parserResult: { searchTerm },
    figState: { buffer, cursorLocation },
    visibleState,
    fuzzySearchEnabled: false,
    settings: { [SETTINGS.INSERT_SPACE_AUTOMATICALLY]: insertSpace },
  }) as unknown as Parameters<typeof inlineSuffix>[1];

const subcommand = (name: string, extra: Partial<Suggestion> = {}) =>
  ({ name, type: "subcommand", ...extra }) as Suggestion;

describe("inlineSuffix", () => {
  it("returns the rest of a prefix match", () => {
    expect(
      inlineSuffix(subcommand("checkout"), makeState("git che", "che")),
    ).toBe("ckout");
  });

  it("returns nothing for a fuzzy match", () => {
    expect(
      inlineSuffix(subcommand("checkout"), makeState("git cot", "cot")),
    ).toBe("");
  });

  it("returns nothing for a case-insensitive match", () => {
    expect(
      inlineSuffix(subcommand("checkout"), makeState("git CHE", "CHE")),
    ).toBe("");
  });

  it("strips a trailing {cursor}", () => {
    expect(
      inlineSuffix(
        subcommand("--message", { separatorToAdd: "=" }),
        makeState("git commit --mes", "--mes"),
      ),
    ).toBe("sage=");
  });

  it("returns nothing for {cursor} in the middle", () => {
    expect(
      inlineSuffix(
        subcommand("commit", { insertValue: "commit -m '{cursor}'" }),
        makeState("git com", "com"),
      ),
    ).toBe("");
  });

  it("returns nothing for an insertion that executes", () => {
    expect(
      inlineSuffix(
        subcommand("status", { insertValue: "status\n" }),
        makeState("git sta", "sta"),
      ),
    ).toBe("");
    expect(
      inlineSuffix(
        subcommand("status", { type: "auto-execute" }),
        makeState("git sta", "sta"),
      ),
    ).toBe("");
  });

  it("adds a space when the setting is on", () => {
    const item = subcommand("checkout", { shouldAddSpace: true });
    expect(inlineSuffix(item, makeState("git che", "che"))).toBe("ckout ");
    expect(
      inlineSuffix(item, makeState("git che", "che", { insertSpace: false })),
    ).toBe("ckout");
  });

  it("escapes a file name with a space", () => {
    expect(
      inlineSuffix(
        { name: "my file.txt", type: "file" },
        makeState("cat my", "my"),
      ),
    ).toBe("\\ file.txt");
  });

  it("returns nothing when the buffer does not end with the query term", () => {
    expect(
      inlineSuffix(subcommand("checkout"), makeState("git che ", "che")),
    ).toBe("");
  });

  it("does not add a space after a trailing {cursor}", () => {
    expect(
      inlineSuffix(
        subcommand("--message", { separatorToAdd: "=", shouldAddSpace: true }),
        makeState("git commit --mes", "--mes"),
      ),
    ).toBe("sage=");
  });

  it("returns nothing for control characters", () => {
    expect(
      inlineSuffix(
        subcommand("status", { insertValue: "status\r" }),
        makeState("git sta", "sta"),
      ),
    ).toBe("");
  });

  it("returns nothing for a shortcut", () => {
    expect(
      inlineSuffix(
        subcommand("deploy", { type: "shortcut" }),
        makeState("dep", "dep"),
      ),
    ).toBe("");
  });

  it("returns nothing for a history item", () => {
    expect(
      inlineSuffix(
        { name: "git checkout main", type: "history" },
        makeState("git che", "che"),
      ),
    ).toBe("");
  });

  it("returns nothing when the cursor is not at the end", () => {
    expect(
      inlineSuffix(
        subcommand("checkout"),
        makeState("git che", "che", { cursorLocation: 3 }),
      ),
    ).toBe("");
  });

  it("returns nothing when suggestions are stale", () => {
    expect(
      inlineSuffix(
        subcommand("checkout"),
        makeState("git che", "che", {
          visibleState: Visibility.HIDDEN_UNTIL_KEYPRESS,
        }),
      ),
    ).toBe("");
  });

  it("returns nothing without an item", () => {
    expect(inlineSuffix(undefined, makeState("git che", "che"))).toBe("");
  });
});

describe("sendInlineSuggestion", () => {
  const command = { tokens: [] };
  let sessionCount = 0;

  const makeSendState = (
    sessionId: string,
    buffer: string,
    { source = "specs", parsedCommand = command as object | null } = {},
  ) =>
    ({
      ...makeState(buffer, "che"),
      figState: {
        buffer,
        cursorLocation: buffer.length,
        shellContext: { sessionId },
      },
      settings: { [SETTINGS.INLINE_SOURCE]: source },
      suggestions: [subcommand("checkout")],
      command,
      parsedCommand,
    }) as unknown as Parameters<typeof sendInlineSuggestion>[0];

  let sessionId = "";
  beforeEach(() => {
    vi.mocked(Shell.setInlineSuggestion).mockClear();
    // The last-sent cache is module-level, so give each test its own session
    sessionCount += 1;
    sessionId = `session-${sessionCount}`;
  });

  it("sends the suffix once per change", () => {
    sendInlineSuggestion(makeSendState(sessionId, "git che"));
    sendInlineSuggestion(makeSendState(sessionId, "git che"));
    expect(Shell.setInlineSuggestion).toHaveBeenCalledTimes(1);
    expect(Shell.setInlineSuggestion).toHaveBeenCalledWith(
      "git che",
      "ckout",
      sessionId,
    );

    sendInlineSuggestion(makeSendState(sessionId, "git chec"));
    expect(Shell.setInlineSuggestion).toHaveBeenCalledTimes(2);
  });

  it("does nothing when spec suggestions are off", () => {
    sendInlineSuggestion(
      makeSendState(sessionId, "git che", { source: "off" }),
    );
    sendInlineSuggestion(
      makeSendState(sessionId, "git che", { source: "history" }),
    );
    expect(Shell.setInlineSuggestion).not.toHaveBeenCalled();
  });

  it("does nothing without a session", () => {
    sendInlineSuggestion(makeSendState("", "git che"));
    expect(Shell.setInlineSuggestion).not.toHaveBeenCalled();
  });

  it("does nothing while the parse is for an older command", () => {
    sendInlineSuggestion(
      makeSendState(sessionId, "git che", { parsedCommand: { tokens: [] } }),
    );
    expect(Shell.setInlineSuggestion).not.toHaveBeenCalled();
  });
});
