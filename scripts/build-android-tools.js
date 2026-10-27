const { spawnSync } = require("node:child_process");
const { mkdirSync } = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const classes = path.join(root, ".tmp/android-tools/classes");
const output = path.join(root, "apps/app/src-tauri/resources/tools/app-info");
const javaHome = process.env.JAVA_HOME;
const binary = (name) => javaHome ? path.join(javaHome, "bin", process.platform === "win32" ? `${name}.exe` : name) : name;
function run(program, args) {
  const result = spawnSync(program, args, { stdio: "inherit", windowsHide: true });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
mkdirSync(classes, { recursive: true });
mkdirSync(output, { recursive: true });
run(binary("javac"), ["--release", "8", "-Xlint:-options", "-encoding", "UTF-8", "-d", classes,
  path.join(root, "apps/app/android-tools/AppLabels.java")]);
run(binary("java"), ["-jar", path.join(root, "apps/app/src-tauri/resources/tools/dex-tools/lib/dx-23.0.0.jar"),
  // The bundled dx predates Java 8; this reflection-only helper uses no Java 8 bytecode features.
  "--dex", "--no-strict", "--output", path.join(output, "app-labels.dex"), classes]);
