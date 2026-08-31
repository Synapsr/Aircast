import { describe, expect, it } from "vitest";
import { normalizedMount, portForTransport, webcastUrl } from "./webcast";

describe("webcastUrl", () => {
  it("uses wss and hides the default port", () => {
    expect(
      webcastUrl({
        host: "stream.radios.bzh",
        port: 443,
        mount: "/webdj/porte-voix/",
      }),
    ).toBe("wss://stream.radios.bzh/webdj/porte-voix/");
  });

  it("keeps a non-default port", () => {
    expect(
      webcastUrl({
        host: "stream.radios.bzh",
        port: 8443,
        mount: "/webdj/porte-voix/",
      }),
    ).toBe("wss://stream.radios.bzh:8443/webdj/porte-voix/");
  });

  it("allows plaintext only on loopback", () => {
    expect(
      webcastUrl({ host: "localhost", port: 8000, mount: "/webdj/x/" }),
    ).toBe("ws://localhost:8000/webdj/x/");
    expect(
      webcastUrl({ host: "127.0.0.1", port: 80, mount: "/webdj/x/" }),
    ).toBe("ws://127.0.0.1/webdj/x/");
    // Anything that leaves the machine must be TLS: the DJ password rides in
    // the first WebSocket frame.
    expect(
      webcastUrl({ host: "example.com", port: 8000, mount: "/webdj/x/" }),
    ).toBe("wss://example.com:8000/webdj/x/");
  });

  it("trims the host and normalizes a missing leading slash", () => {
    expect(
      webcastUrl({ host: "  example.com  ", port: 443, mount: "webdj/x/" }),
    ).toBe("wss://example.com/webdj/x/");
  });
});

describe("normalizedMount", () => {
  it("adds a leading slash when missing", () => {
    expect(normalizedMount("webdj/x/")).toBe("/webdj/x/");
    expect(normalizedMount("/webdj/x/")).toBe("/webdj/x/");
  });
});

describe("portForTransport", () => {
  it("moves a real AzuraCast source port to 443", () => {
    // The DJ/harbor port is 8005, 8015, 8025… — never the 8000 default, so a
    // rule keyed on defaults would never fire for a real preset.
    for (const p of [8005, 8015, 8025, 8255, 8000]) {
      expect(portForTransport(p, "webcast")).toBe(443);
    }
  });

  it("keeps a port that is already plausible for TLS", () => {
    expect(portForTransport(443, "webcast")).toBe(443);
    expect(portForTransport(8443, "webcast")).toBe(8443);
    expect(portForTransport(80, "webcast")).toBe(80);
  });

  it("moves a TLS port back to the icecast default", () => {
    expect(portForTransport(443, "icecast")).toBe(8000);
    expect(portForTransport(8443, "icecast")).toBe(8000);
  });

  it("never clobbers a deliberate icecast port", () => {
    for (const p of [8005, 8255, 9000, 80]) {
      expect(portForTransport(p, "icecast")).toBe(p);
    }
  });
});
