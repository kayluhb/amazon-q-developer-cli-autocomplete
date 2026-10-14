/**
 * Build a quoted CSS `url()` value. Quoting keeps characters such as `)`
 * inside the value. Escaping keeps `"`, `\` and newlines from ending the string.
 */
export const cssUrl = (url: URL | string): string =>
  `url("${String(url).replace(
    /["\\\n\r\f]/g,
    (char) => `\\${char.charCodeAt(0).toString(16)} `,
  )}")`;
