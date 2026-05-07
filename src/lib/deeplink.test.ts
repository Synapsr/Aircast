import { describe, expect, it } from "vitest";
import { parseServerLink, uniquePresetName } from "./deeplink";

describe("parseServerLink", () => {
  it("parses a fully-specified URL", () => {
    const url =
      "aircast://add-server?name=Prod&host=stream.example.com&port=8000&mount=/live.mp3&user=source&pass=secret&format=mp3&bitrate=128";
    const parsed = parseServerLink(url, "device-1");
    expect(parsed).not.toBeNull();
    expect(parsed!.name).toBe("Prod");
    expect(parsed!.config).toMatchObject({
      deviceId: "device-1",
      host: "stream.example.com",
      port: 8000,
      mount: "/live.mp3",
      username: "source",
      password: "secret",
      format: "mp3",
      bitrate: 128,
    });
  });

  it("rejects URLs that aren't aircast scheme", () => {
    expect(parseServerLink("https://example.com/?host=foo", "")).toBeNull();
    expect(parseServerLink("file:///tmp/foo", "")).toBeNull();
  });

  it("rejects unknown actions", () => {
    expect(
      parseServerLink("aircast://other-action?host=foo", ""),
    ).toBeNull();
  });

  it("rejects URLs without host", () => {
    expect(
      parseServerLink("aircast://add-server?port=8000", ""),
    ).toBeNull();
  });

  it("rejects out-of-range ports", () => {
    expect(
      parseServerLink(
        "aircast://add-server?host=foo&port=0",
        "",
      ),
    ).toBeNull();
    expect(
      parseServerLink(
        "aircast://add-server?host=foo&port=99999",
        "",
      ),
    ).toBeNull();
    expect(
      parseServerLink(
        "aircast://add-server?host=foo&port=-1",
        "",
      ),
    ).toBeNull();
  });

  it("normalizes mount without leading slash", () => {
    const parsed = parseServerLink(
      "aircast://add-server?host=foo&mount=live.mp3",
      "",
    );
    expect(parsed!.config.mount).toBe("/live.mp3");
  });

  it("falls back to defaults for missing fields", () => {
    const parsed = parseServerLink("aircast://add-server?host=stream.example.com", "");
    expect(parsed!.config.port).toBe(8000);
    expect(parsed!.config.username).toBe("source");
    expect(parsed!.config.format).toBe("mp3");
    expect(parsed!.config.bitrate).toBe(128);
  });

  it("falls back to host as name when name is missing", () => {
    const parsed = parseServerLink("aircast://add-server?host=stream.example.com", "");
    expect(parsed!.name).toBe("stream.example.com");
  });

  it("rejects bitrates outside the supported set", () => {
    // 100 isn't in [64, 128, 192, 320] → should fall back to default 128
    const parsed = parseServerLink(
      "aircast://add-server?host=foo&bitrate=100",
      "",
    );
    expect(parsed!.config.bitrate).toBe(128);
  });

  it("accepts only mp3 or aac for format", () => {
    const a = parseServerLink("aircast://add-server?host=foo&format=aac", "");
    expect(a!.config.format).toBe("aac");
    const b = parseServerLink("aircast://add-server?host=foo&format=ogg", "");
    expect(b!.config.format).toBe("mp3"); // unsupported → default
  });

  it("preserves deviceId from caller", () => {
    const parsed = parseServerLink("aircast://add-server?host=foo", "my-mic");
    expect(parsed!.config.deviceId).toBe("my-mic");
  });

  it("returns null for malformed URLs", () => {
    expect(parseServerLink("not a url at all", "")).toBeNull();
    expect(parseServerLink("", "")).toBeNull();
  });

  it("decodes URL-encoded credentials", () => {
    const parsed = parseServerLink(
      "aircast://add-server?host=foo&user=us%40er&pass=p%40ss",
      "",
    );
    expect(parsed!.config.username).toBe("us@er");
    expect(parsed!.config.password).toBe("p@ss");
  });
});

describe("uniquePresetName", () => {
  it("returns the base name when not taken", () => {
    expect(uniquePresetName("Server", [])).toBe("Server");
    expect(uniquePresetName("Server", ["Other"])).toBe("Server");
  });

  it("appends ' (2)' when base is taken", () => {
    expect(uniquePresetName("Server", ["Server"])).toBe("Server (2)");
  });

  it("increments the suffix until a free slot is found", () => {
    expect(
      uniquePresetName("Server", ["Server", "Server (2)", "Server (3)"]),
    ).toBe("Server (4)");
  });
});
