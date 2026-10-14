// @ts-check
import react from "@tab/eslint-config/react";
export default [
  { ignores: ["dist"] },
  ...react({
    tsconfigPath: "./tsconfig.json",
  }),
];
