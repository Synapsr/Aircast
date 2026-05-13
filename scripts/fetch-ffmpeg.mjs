#!/usr/bin/env node
// Downloads a minimal LGPL ffmpeg build for the host platform and places it
// at src-tauri/binaries/aircast-ffmpeg-<rust-target-triple>, which is the
// layout Tauri's `bundle.externalBin` expects. The `aircast-` prefix is
// load-bearing on Linux: Tauri's deb bundler installs sidecars to
// /usr/bin/<base-name>, and a bare `ffmpeg` name would collide with the
// system ffmpeg package on Ubuntu 24.10+, breaking dpkg install.
//
// Sources:
//   - Linux x86_64:   BtbN/FFmpeg-Builds (lgpl, static)
//   - Linux aarch64:  BtbN/FFmpeg-Builds (lgpl, static)
//   - Windows x86_64: BtbN/FFmpeg-Builds (lgpl, static)
//   - macOS:          evermeet.cx (universal static, no GitHub release for macOS)
//
// If a build is missing for the host platform, the script prints a helpful
// message and exits non-zero. v1: only attempts the host platform; CI handles
// cross-platform.

import { execSync } from "node:child_process";
import { mkdirSync, existsSync, chmodSync, renameSync, rmSync, readdirSync, statSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const binDir = resolve(root, "src-tauri/binaries");
const tmpDir = resolve(root, "src-tauri/.ffmpeg-tmp");

const platform = os.platform();
const arch = os.arch();

const PLAN = (() => {
  if (platform === "darwin" && arch === "arm64") {
    return {
      triple: "aarch64-apple-darwin",
      url: "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip",
      archiveType: "zip",
      binaryRelative: ["ffmpeg"],
    };
  }
  if (platform === "darwin" && arch === "x64") {
    return {
      triple: "x86_64-apple-darwin",
      url: "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip",
      archiveType: "zip",
      binaryRelative: ["ffmpeg"],
    };
  }
  if (platform === "linux" && arch === "x64") {
    return {
      triple: "x86_64-unknown-linux-gnu",
      url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-lgpl.tar.xz",
      archiveType: "tar.xz",
      binaryRelative: ["bin/ffmpeg", "ffmpeg"],
    };
  }
  if (platform === "linux" && arch === "arm64") {
    return {
      triple: "aarch64-unknown-linux-gnu",
      url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-lgpl.tar.xz",
      archiveType: "tar.xz",
      binaryRelative: ["bin/ffmpeg", "ffmpeg"],
    };
  }
  if (platform === "win32" && arch === "x64") {
    return {
      triple: "x86_64-pc-windows-msvc",
      url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip",
      archiveType: "zip",
      binaryRelative: ["bin/ffmpeg.exe"],
    };
  }
  return null;
})();

if (!PLAN) {
  console.error(`Unsupported host platform: ${platform}/${arch}.`);
  console.error("Supported: macOS arm64/x64, Linux x86_64/aarch64, Windows x64.");
  process.exit(1);
}

const targetExt = platform === "win32" ? ".exe" : "";
const targetName = `aircast-ffmpeg-${PLAN.triple}${targetExt}`;
const targetPath = join(binDir, targetName);

if (existsSync(targetPath)) {
  console.log(`✓ ${targetName} already present.`);
  process.exit(0);
}

mkdirSync(binDir, { recursive: true });
if (existsSync(tmpDir)) {
  rmSync(tmpDir, { recursive: true, force: true });
}
mkdirSync(tmpDir, { recursive: true });

const archivePath = join(
  tmpDir,
  PLAN.archiveType === "zip" ? "ffmpeg.zip" : "ffmpeg.tar.xz",
);

console.log(`→ Downloading ${PLAN.url}`);
run(`curl -L --fail --progress-bar -o "${archivePath}" "${PLAN.url}"`);

console.log(`→ Extracting…`);
if (PLAN.archiveType === "zip") {
  run(`unzip -q "${archivePath}" -d "${tmpDir}"`);
} else {
  run(`tar -xf "${archivePath}" -C "${tmpDir}"`);
}

const binarySrc = findBinary(tmpDir, PLAN.binaryRelative);
if (!binarySrc) {
  console.error("Could not locate ffmpeg binary inside the archive.");
  process.exit(2);
}

renameSync(binarySrc, targetPath);
if (platform !== "win32") {
  chmodSync(targetPath, 0o755);
}

rmSync(tmpDir, { recursive: true, force: true });

console.log(`✓ Placed ${targetName} (${humanBytes(statSync(targetPath).size)})`);

function run(cmd) {
  execSync(cmd, { stdio: "inherit" });
}

function findBinary(rootDir, relativeCandidates) {
  // evermeet zips extract the binary directly at root.
  for (const rel of relativeCandidates) {
    const candidate = join(rootDir, rel);
    if (existsSync(candidate)) return candidate;
  }
  // BtbN archives have a single top-level dir; walk one level.
  const entries = readdirSync(rootDir, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const subdir = join(rootDir, entry.name);
    for (const rel of relativeCandidates) {
      const candidate = join(subdir, rel);
      if (existsSync(candidate)) return candidate;
    }
    const direct = join(subdir, platform === "win32" ? "ffmpeg.exe" : "ffmpeg");
    if (existsSync(direct)) return direct;
  }
  return null;
}

function humanBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
