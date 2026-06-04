import { mkdirSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const cacheHome = join(tmpdir(), "codex-cache");
mkdirSync(cacheHome, { recursive: true });
const pnpmHome = process.env.PNPM_HOME ?? join(homedir(), ".local", "share", "pnpm");
const pnpmBin = join(pnpmHome, "pnpm");

const env = {
  ...process.env,
  XDG_CACHE_HOME: cacheHome,
};

run(pnpmBin, ["build:sidecar:windows"], env);
run(pnpmBin, ["tauri", "build", "--runner", "cargo-xwin", "--target", "x86_64-pc-windows-msvc"], env);

function run(command, args, env) {
  const result = spawnSync(command, args, {
    env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
