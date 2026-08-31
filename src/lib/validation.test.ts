import { describe, expect, it } from "vitest";
import { validateStreamConfig } from "./validation";
import type { StreamConfig } from "@/types";

const valid: StreamConfig = {
  deviceId: "device-1",
  host: "stream.example.com",
  port: 8000,
  mount: "/live.mp3",
  username: "source",
  password: "secret",
  bitrate: 128,
  format: "mp3",
  transport: "icecast",
};

describe("validateStreamConfig", () => {
  it("accepts a webcast config", () => {
    expect(
      validateStreamConfig({
        ...valid,
        transport: "webcast",
        port: 443,
        mount: "/webdj/my-station/",
        username: "loan",
      }),
    ).toBeNull();
  });

  it("rejects a webcast 'source' login whose password would be mis-split", () => {
    // AzuraCast splits on ',' before ':', so either character breaks it.
    expect(
      validateStreamConfig({
        ...valid,
        transport: "webcast",
        username: "source",
        password: "a,b",
      }),
    ).toBe("errors.webdjSourceSplit");
    expect(
      validateStreamConfig({
        ...valid,
        transport: "webcast",
        username: "source",
        password: "a:b",
      }),
    ).toBe("errors.webdjSourceSplit");
  });

  it("leaves icecast configs alone when the password has separators", () => {
    expect(
      validateStreamConfig({ ...valid, username: "source", password: "a:b" }),
    ).toBeNull();
  });

  it("accepts a fully-valid config", () => {
    expect(validateStreamConfig(valid)).toBeNull();
  });

  it("rejects null config", () => {
    expect(validateStreamConfig(null)).toBe("errors.noServer");
  });

  it("rejects empty deviceId", () => {
    expect(validateStreamConfig({ ...valid, deviceId: "" })).toBe(
      "errors.noDevice",
    );
  });

  it("rejects empty host", () => {
    expect(validateStreamConfig({ ...valid, host: "" })).toBe(
      "errors.missingHost",
    );
    expect(validateStreamConfig({ ...valid, host: "   " })).toBe(
      "errors.missingHost",
    );
  });

  it("rejects out-of-range ports", () => {
    expect(validateStreamConfig({ ...valid, port: 0 })).toBe(
      "errors.badPort",
    );
    expect(validateStreamConfig({ ...valid, port: 65536 })).toBe(
      "errors.badPort",
    );
    expect(validateStreamConfig({ ...valid, port: -1 })).toBe(
      "errors.badPort",
    );
  });

  it("accepts edge ports 1 and 65535", () => {
    expect(validateStreamConfig({ ...valid, port: 1 })).toBeNull();
    expect(validateStreamConfig({ ...valid, port: 65535 })).toBeNull();
  });

  it("rejects empty mount", () => {
    expect(validateStreamConfig({ ...valid, mount: "" })).toBe(
      "errors.missingMount",
    );
  });

  it("requires mount to start with a slash", () => {
    expect(validateStreamConfig({ ...valid, mount: "live.mp3" })).toBe(
      "errors.mountSlash",
    );
  });

  it("rejects empty username", () => {
    expect(validateStreamConfig({ ...valid, username: "" })).toBe(
      "errors.missingUsername",
    );
  });

  it("accepts empty password", () => {
    // Some Icecast instances allow empty password; we don't reject it.
    expect(validateStreamConfig({ ...valid, password: "" })).toBeNull();
  });
});
