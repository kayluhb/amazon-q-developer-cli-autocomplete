import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import SuggestionIcon from "../SuggestionIcon";
import { cssUrl } from "../cssUrl";

const renderedStyle = (element: React.ReactElement) => {
  const container = document.createElement("div");
  container.innerHTML = renderToStaticMarkup(element);
  return (
    container.querySelector<HTMLElement>("[role=img]")?.getAttribute("style") ??
    ""
  );
};

beforeAll(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).fig = { constants: { newUriFormat: true } };
});

describe("cssUrl", () => {
  it("quotes fig:// URLs", () => {
    expect(cssUrl(new URL("fig://icon?type=box"))).toBe(
      'url("fig://icon?type=box")',
    );
  });

  it("escapes quotes, backslashes and newlines", () => {
    expect(cssUrl('a"b\\c\nd')).toBe('url("a\\22 b\\5c c\\a d")');
  });
});

describe("SuggestionIcon", () => {
  it("keeps a crafted file name inside one quoted url()", () => {
    const style = renderedStyle(
      <SuggestionIcon
        suggestion={{ name: "x),url(https://evil.example/a", type: "file" }}
        iconPath="/tmp/"
        style={{ height: 10 }}
      />,
    );
    expect(style).toContain(
      'background-image:url("fig://path//tmp/x),url(https://evil.example/a")',
    );
  });
});
