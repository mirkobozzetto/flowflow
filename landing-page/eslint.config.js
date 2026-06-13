import astro from "eslint-plugin-astro";
import tseslint from "typescript-eslint";

export default [
  ...astro.configs.recommended,
  {
    files: ["**/*.astro"],
    plugins: { "@typescript-eslint": tseslint.plugin },
    rules: {
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          args: "after-used",
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          ignoreRestSiblings: true,
        },
      ],
    },
  },
  {
    ignores: ["dist/", ".astro/", "node_modules/"],
  },
];
