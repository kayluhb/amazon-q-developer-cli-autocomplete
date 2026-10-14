// Types are in csp.d.ts.

const DEVELOPMENT_MODE = "development";

export function htmlCspPlugin(csp) {
  const content = Object.entries(csp)
    .map(([directive, value]) => `${directive} ${value}`)
    .join("; ");

  return {
    name: "html-csp",
    transformIndexHtml: {
      order: "post",
      handler: (_html, ctx) => {
        if (ctx.server?.config?.mode === DEVELOPMENT_MODE) {
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
