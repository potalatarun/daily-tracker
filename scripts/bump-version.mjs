#!/usr/bin/env node
// Sets the app version everywhere it lives, so a release tag always matches.
// Usage: npm run version:set 0.2.0
import { readFileSync, writeFileSync } from "node:fs";

const v = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(v ?? "")) {
  console.error("Usage: npm run version:set <major.minor.patch>   e.g. 0.2.0");
  process.exit(1);
}

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
pkg.version = v;
writeFileSync("package.json", JSON.stringify(pkg, null, 2) + "\n");

const lockPath = "package-lock.json";
const lock = JSON.parse(readFileSync(lockPath, "utf8"));
lock.version = v;
if (lock.packages?.[""]) lock.packages[""].version = v;
writeFileSync(lockPath, JSON.stringify(lock, null, 2) + "\n");

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8").replace(/^version = ".*"$/m, `version = "${v}"`);
writeFileSync(cargoPath, cargo);

console.log(`Version set to ${v}. Commit, then: git tag v${v} && git push origin v${v}`);
