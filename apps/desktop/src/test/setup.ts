/* ==========================================================================
 * Vitest setup
 * Extends matchers with jest-dom, unmounts rendered trees between tests
 * (Testing Library only auto-cleans when globals are enabled, which this
 * project does not use), and polyfills matchMedia — jsdom does not implement
 * it but the theme hook depends on it.
 * ========================================================================== */

import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
});

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    addListener: () => undefined,
    removeListener: () => undefined,
    dispatchEvent: () => false,
  }),
});
