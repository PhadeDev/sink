// Tailwind CSS v4 is configured CSS-first in src/styles/globals.css.
// This file exists to satisfy the target file structure in SPEC.md and to
// host any future JS-side configuration (via `@config` in globals.css).
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
} satisfies Config;
