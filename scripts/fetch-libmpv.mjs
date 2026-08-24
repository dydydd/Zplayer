// Downloads the platform-specific libmpv runtime at build time so the binaries
// never need to live in the repository. Mirrors the sources recorded in
// libmpv/SOURCES.md.
//
//   node scripts/fetch-libmpv.mjs
//
// Windows needs 7-Zip on PATH (or at the usual Program Files location).
// macOS uses the system `tar`. Linux uses the system libmpv package instead of
// a bundled runtime, so this script is a no-op there.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readdirSync,
  copyFileSync,
  readFileSync,
  writeFileSync,
  renameSync,
  rmSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");
const libmpvDir = join(repoRoot, "libmpv");

const PLATFORMS = {
  "win32-x64": {
    dest: "windows-x86_64",
    url: "https://github.com/media-kit/libmpv-win32-video-build/releases/download/2023-09-24/mpv-dev-x86_64-20230924-git-652a1dd.7z",
    sha256: "dce982222d7a23e4a1c6f0fb6cc39f6e899a6714624b95ea49cff6558ee97572",
    kind: "7z-dll",
  },
  "darwin-x64": null,
  "darwin-arm64": {
    dest: "macos-universal",
    url: "https://github.com/media-kit/libmpv-darwin-build/releases/download/v0.6.0/libmpv-xcframeworks_v0.6.0_macos-universal-video-default.tar.gz",
    sha256: "84d2ad98e046e82c6dc34d8547d76c2afeaee89c0f53032773be8985c95536d6",
    kind: "tar-xcframework",
  },
  "linux-x64": {
    dest: "linux-x86_64",
    systemOnly: true,
  },
};

function platformKey() {
  const arch = process.arch === "arm64" ? "arm64" : "x64";
  return `${process.platform}-${arch}`;
}

function fail(message) {
  console.error(`fetch-libmpv: ${message}`);
  process.exit(1);
}

function sha256File(path) {
  const hash = createHash("sha256");
  hash.update(readFileSync(path));
  return hash.digest("hex");
}

function find7z() {
  const candidates = [
    "7z",
    "7za",
    "7zr",
    "C:\\Program Files\\7-Zip\\7z.exe",
    "C:\\Program Files (x86)\\7-Zip\\7z.exe",
  ];
  for (const candidate of candidates) {
    if (candidate.includes("\\") || candidate.includes("/")) {
      if (existsSync(candidate)) return candidate;
      continue;
    }
    try {
      const out = execFileSync("where", [candidate], { stdio: "ignore" });
      if (out) return candidate;
    } catch {
      // not on PATH
    }
  }
  return null;
}

async function download(url, destPath) {
  console.log(`fetch-libmpv: downloading ${url}`);
  const tmp = destPath + ".download";
  const response = await fetch(url);
  if (!response.ok) fail(`download failed: HTTP ${response.status} for ${url}`);
  const arrayBuffer = await response.arrayBuffer();
  mkdirSync(dirname(tmp), { recursive: true });
  writeFileSync(tmp, Buffer.from(arrayBuffer));
  renameSync(tmp, destPath);
}

function extract7z(archive, outDir) {
  console.log("fetch-libmpv: extracting 7z archive");
  const sevenZip = find7z();
  if (!sevenZip) {
    fail("7-Zip was not found. Install 7-Zip and ensure 7z.exe is on PATH, then retry.");
  }
  mkdirSync(outDir, { recursive: true });
  execFileSync(sevenZip, ["x", "-y", `-o${outDir}`, archive], { stdio: "inherit" });
}

function copyDlls(fromDir, toDir) {
  mkdirSync(toDir, { recursive: true });
  let count = 0;
  for (const entry of readdirSync(fromDir, { withFileTypes: true })) {
    const full = join(fromDir, entry.name);
    if (entry.isDirectory()) {
      count += copyDlls(full, toDir);
    } else if (entry.name.toLowerCase().endsWith(".dll")) {
      copyFileSync(full, join(toDir, entry.name));
      count++;
    }
  }
  return count;
}

async function run() {
  const key = platformKey();
  const config = PLATFORMS[key];
  if (!config) {
    fail(
      `unsupported platform ${key} (process.platform=${process.platform}, process.arch=${process.arch})`
    );
  }

  if (config.systemOnly) {
    console.log(
      "fetch-libmpv: Linux uses the system libmpv package; nothing to fetch. " +
        "Install it with `apt-get install -y libmpv-dev` before building."
    );
    return;
  }

  const destDir = join(libmpvDir, config.dest);
  const sentinel = config.kind === "7z-dll"
    ? join(destDir, "libmpv-2.dll")
    : join(destDir, "Mpv.xcframework");
  if (existsSync(sentinel)) {
    console.log(`fetch-libmpv: ${config.dest} already populated, skipping download.`);
    return;
  }

  const tmpDir = join(repoRoot, "node_modules", ".cache", "zplayer-libmpv");
  mkdirSync(tmpDir, { recursive: true });
  const archive = join(tmpDir, "libmpv-archive");

  if (!existsSync(archive)) {
    await download(config.url, archive);
  }

  const actual = sha256File(archive);
  if (actual.toLowerCase() !== config.sha256.toLowerCase()) {
    fail(
      `SHA256 mismatch for ${archive}\n  expected ${config.sha256}\n  actual   ${actual}`
    );
  }

  const extractDir = join(tmpDir, "extracted");
  if (existsSync(extractDir)) {
    rmSync(extractDir, { recursive: true, force: true });
  }
  mkdirSync(extractDir, { recursive: true });

  if (config.kind === "7z-dll") {
    extract7z(archive, extractDir);
    const copied = copyDlls(extractDir, destDir);
    console.log(`fetch-libmpv: copied ${copied} DLL(s) into ${destDir}`);
  } else if (config.kind === "tar-xcframework") {
    // Extract directly into the destination, dropping the archive's single top
    // level directory. `tar` recreates the framework symlinks (e.g.
    // Foo.framework/Resources -> Versions/A/Resources), which a manual file
    // copy cannot do.
    mkdirSync(destDir, { recursive: true });
    execFileSync(
      "tar",
      ["-xzf", archive, "-C", destDir, "--strip-components=1"],
      { stdio: "inherit" }
    );
    console.log(`fetch-libmpv: extracted xcframeworks into ${destDir}`);
  }

  console.log("fetch-libmpv: done.");
}

run().catch((err) => fail(err && err.message ? err.message : String(err)));
