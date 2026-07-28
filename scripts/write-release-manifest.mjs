import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";

function command(program, args = []) {
  const executable =
    process.platform === "win32" && program === "npm" ? "npm.cmd" : program;
  return execFileSync(executable, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function firstLine(value) {
  return value.split(/\r?\n/, 1)[0];
}

function npmVersion() {
  const supplied = process.env.SKCF_RELEASE_NPM_VERSION?.trim();
  if (supplied) {
    return supplied;
  }
  if (process.platform === "win32") {
    throw new Error(
      "SKCF_RELEASE_NPM_VERSION is required when writing a release manifest on Windows",
    );
  }
  return command("npm", ["--version"]);
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function argumentValues(flag) {
  const values = [];
  for (let index = 2; index < process.argv.length; index += 1) {
    if (process.argv[index] === flag && process.argv[index + 1]) {
      values.push(process.argv[index + 1]);
      index += 1;
    }
  }
  return values;
}

const outputPath = argumentValues("--output")[0];
const artifactPaths = argumentValues("--artifact").map((path) => resolve(path));
if (!outputPath || artifactPaths.length === 0) {
  throw new Error(
    "Usage: node scripts/write-release-manifest.mjs --output <json> --artifact <file> [--artifact <file>]",
  );
}

const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const appVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!appVersion) {
  throw new Error("Cargo.toml did not report the application version");
}
const targetTriple = command("rustc", ["-vV"])
  .split(/\r?\n/)
  .find((line) => line.startsWith("host: "))
  ?.slice(6);
if (!targetTriple) {
  throw new Error("rustc did not report a target triple");
}

const manifest = {
  formatVersion: 1,
  appVersion,
  commit: command("git", ["rev-parse", "HEAD"]),
  targetTriple,
  buildTimestamp: new Date().toISOString(),
  buildMode: "internal-unsigned-debug",
  signed: false,
  projectSchemaVersion: 1,
  tools: {
    rustc: firstLine(command("rustc", ["--version"])),
    node: process.version,
    npm: npmVersion(),
    ffmpeg: firstLine(command("ffmpeg", ["-version"])),
    ffprobe: firstLine(command("ffprobe", ["-version"])),
  },
  bundledSidecars: false,
  sidecars: [],
  artifacts: artifactPaths.map((path) => {
    const metadata = statSync(path);
    if (!metadata.isFile() || metadata.size === 0) {
      throw new Error(`Release artifact is missing or empty: ${path}`);
    }
    return {
      filename: basename(path),
      sizeBytes: metadata.size,
      sha256: sha256(path),
    };
  }),
};

writeFileSync(resolve(outputPath), `${JSON.stringify(manifest, null, 2)}\n`, {
  flag: "wx",
});
