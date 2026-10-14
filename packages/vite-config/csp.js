// Types are in csp.d.ts.

export function htmlCspPlugin(csp) {
  const content = Object.entries(csp)
    .map(([k, v]) => `${k} ${v}`)
    .join("; ");

  return {
    name: "html-csp",
    transformIndexHtml: {
      order: "post",
      handler: (_html, ctx) => {
        if (ctx.server?.config?.mode === "development") {
          return [];
        }

        return [
          {
            injectTo: "head",
            tag: "meta",
            attrs: {
              "http-equiv": "Content-Security-Policy",
              content,
            },
          },
        ];
      },
    },
  };
}
