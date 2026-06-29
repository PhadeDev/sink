// Tailwind CSS v4 is configured CSS-first in src/styles/globals.css.
// This near-empty file is kept as the conventional home for any future
// JS-side configuration (referenced via `@config` in globals.css).
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
} satisfies Config;
