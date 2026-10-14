import type { Plugin } from "vite";

/**
 * Adds a Content-Security-Policy meta tag with the given directives to
 * index.html. The dev server does not add the tag.
 */
export declare function htmlCspPlugin(csp: Record<string, string>): Plugin;
