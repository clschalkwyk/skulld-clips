import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const manifestLines = readFileSync("MANIFEST.sha256", "utf8")
  .split(/\r?\n/)
  .filter(Boolean);

for (const line of manifestLines) {
  const match = line.match(/^([0-9a-f]{64}) {2}(.+)$/);
  if (!match) {
    throw new Error(`Malformed checksum manifest line: ${line}`);
  }
  const [, expected, path] = match;
  const bytes = readFileSync(path);
  const lf = Buffer.from(bytes.toString("utf8").replace(/\r\n/g, "\n"));
  const crlf = Buffer.from(lf.toString("utf8").replace(/\n/g, "\r\n"));
  const hashes = [bytes, lf, crlf].map((content) =>
    createHash("sha256").update(content).digest("hex"),
  );
  if (!hashes.includes(expected)) {
    throw new Error(`Checksum mismatch: ${path}`);
  }
  console.log(`${path}: OK`);
}

for (const path of [
  "spec-manifest.json",
  "contracts/project.schema.json",
  "contracts/export-events.schema.json",
  "examples/example-project.skcf.json",
  "release/RELEASE_MANIFEST.schema.json",
]) {
  JSON.parse(readFileSync(path, "utf8"));
}

console.log("Specification and contract JSON: OK");
