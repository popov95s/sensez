import tsParser from "@typescript-eslint/parser";

export default [
  {
    ignores: ["node_modules/**", ".ruff_cache/**"],
  },
  {
    files: ["**/*.ts"],
    languageOptions: {
      ecmaVersion: "latest",
      parser: tsParser,
      sourceType: "module",
    },
    rules: {
      "comma-dangle": [
        "error",
        {
          arrays: "always-multiline",
          objects: "always-multiline",
          imports: "always-multiline",
          exports: "always-multiline",
          functions: "always-multiline",
        },
      ],
      quotes: ["error", "double"],
      semi: ["error", "always"],
    },
  },
];
