import nextVitals from "eslint-config-next/core-web-vitals";
import nextTypescript from "eslint-config-next/typescript";

const config = [
  ...nextVitals,
  ...nextTypescript,
  {
    rules: {
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "react",
              importNames: ["useEffect"],
              message:
                "useEffect is prohibited by project standards. Use Server Components, event handlers, or a dedicated sync hook.",
            },
          ],
        },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector: "CallExpression[callee.name='useEffect']",
          message:
            "useEffect is prohibited by project standards. Use Server Components, event handlers, or a dedicated sync hook.",
        },
        {
          selector: "CallExpression[callee.property.name='useEffect']",
          message:
            "useEffect is prohibited by project standards. Use Server Components, event handlers, or a dedicated sync hook.",
        },
      ],
    },
  },
];

export default config;
