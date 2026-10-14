import { describe, expect, it } from "vitest";
import { SETTINGS } from "@tab/api-bindings-wrappers";
import { Suggestion } from "@tab/shared/internal";
import { inlineSuffix } from "../inlineSuggestion";
import { Visibility } from "../types";

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
      inlineSuffix(
        { name: "my file.txt", type: "file" },
        makeState('cat "my fi', "my fi"),
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
