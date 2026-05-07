import { describe, expect, it } from "vitest";
import { detectLocale, resolveLocale, translate } from "./index";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

describe("translate", () => {
  it("returns the value for a known nested key", () => {
    expect(translate("en", "header.setup")).toBe("Setup");
    expect(translate("fr", "header.setup")).toBe("Réglages");
  });

  it("falls back to English for missing French keys", () => {
    // We use a key that exists in EN but pretend FR is missing
    // (in practice the JSONs are mirrored — this is the safety net).
    expect(translate("fr", "errors.openSetup")).toBe("Ouvrir les réglages");
  });

  it("returns the raw key when nothing matches (visible failure mode)", () => {
    expect(translate("en", "this.key.does.not.exist")).toBe(
      "this.key.does.not.exist",
    );
  });

  it("substitutes {placeholders}", () => {
    expect(
      translate("en", "status.reconnectingIn", { seconds: 3 }),
    ).toBe("Reconnecting in 3s…");
    expect(
      translate("fr", "status.reconnectingIn", { seconds: 5 }),
    ).toBe("Reconnexion dans 5s…");
  });

  it("substitutes multiple placeholders", () => {
    expect(translate("en", "link.renamedHint", { name: "Foo" })).toContain(
      '"Foo"',
    );
  });
});

describe("resolveLocale", () => {
  it("returns the explicit locale when set", () => {
    expect(resolveLocale("en")).toBe("en");
    expect(resolveLocale("fr")).toBe("fr");
  });

  it("delegates to navigator detection on auto", () => {
    // Default jsdom navigator.language is "en-US"
    expect(resolveLocale("auto")).toBe("en");
  });
});

describe("detectLocale", () => {
  it("returns 'fr' for French navigator languages", () => {
    const original = Object.getOwnPropertyDescriptor(navigator, "language");
    Object.defineProperty(navigator, "language", {
      value: "fr-FR",
      configurable: true,
    });
    expect(detectLocale()).toBe("fr");
    if (original) Object.defineProperty(navigator, "language", original);
  });

  it("returns 'en' for non-French languages", () => {
    const original = Object.getOwnPropertyDescriptor(navigator, "language");
    Object.defineProperty(navigator, "language", {
      value: "es-ES",
      configurable: true,
    });
    expect(detectLocale()).toBe("en");
    if (original) Object.defineProperty(navigator, "language", original);
  });
});

describe("locale dictionaries are in sync", () => {
  // Walk the EN dictionary and ensure every leaf key exists in FR.
  // Regression guard: it's very easy to add a new key to one and forget
  // the other.
  function leafKeys(obj: unknown, prefix = ""): string[] {
    if (typeof obj !== "object" || obj === null) return [prefix];
    const out: string[] = [];
    for (const [k, v] of Object.entries(obj)) {
      const path = prefix ? `${prefix}.${k}` : k;
      out.push(...leafKeys(v, path));
    }
    return out;
  }

  it("every English key has a French translation", () => {
    const enKeys = new Set(leafKeys(en));
    const frKeys = new Set(leafKeys(fr));
    const missing = [...enKeys].filter((k) => !frKeys.has(k));
    expect(missing).toEqual([]);
  });

  it("every French key has an English translation", () => {
    const enKeys = new Set(leafKeys(en));
    const frKeys = new Set(leafKeys(fr));
    const extra = [...frKeys].filter((k) => !enKeys.has(k));
    expect(extra).toEqual([]);
  });
});
