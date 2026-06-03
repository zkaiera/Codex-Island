import { copyFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const target = "x86_64-pc-windows-msvc";
const source = join("src-tauri", "target", target, "release", "codex-island-hook.exe");
const destination = join("src-tauri", "binaries", `codex-island-hook-${target}.exe`);
const runner = selectRunner();

mkdirSync(dirname(destination), { recursive: true });
if (!existsSync(destination)) {
  writeFileSync(destination, "");
}

run(runner, [
  "build",
  "--manifest-path",
  join("src-tauri", "Cargo.toml"),
  "--bin",
  "codex-island-hook",
  "--release",
  "--target",
  target,
]);

copyFileSync(source, destination);
console.log(`sidecar ready: ${destination}`);

function selectRunner() {
  if (process.platform === "win32") {
    return "cargo";
  }

  return commandExists("cargo-xwin") ? "cargo-xwin" : "cargo";
}

function commandExists(command) {
  const result = spawnSync(command, ["--version"], { stdio: "ignore" });
  return result.status === 0;
}

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  if (!existsSync(source)) {
    console.error(`missing expected sidecar binary: ${source}`);
    process.exit(1);
  }
}
