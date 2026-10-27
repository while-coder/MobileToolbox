const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const ts = require("../apps/app/node_modules/typescript");

function load(name, native) {
  const source = fs.readFileSync(path.join(__dirname, "../apps/app/src/platform", name + ".ts"), "utf8");
  const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const module = { exports: {} };
  new Function("require", "module", "exports", code)(name => {
    assert.equal(name, "./native");
    return { native };
  }, module, module.exports);
  return module.exports;
}
const output = (stdout = "", stderr = "") => ({ stdout, stderr });

test("ADB parsers preserve offline devices and reject package-manager errors", () => {
  const { parseDevices, parsePackages } = load("adb", {});
  assert.deepEqual(parsePackages("package:android\r\npackage:com.example.app\r\npackage:com.example.app\n"), ["android", "com.example.app"]);
  assert.throws(() => parsePackages("Error: device offline"));
  const devices = parseDevices("List of devices attached\r\nemulator-5554 device product:pangu model:PHY110 transport_id:1\r\nphone unauthorized\n");
  assert.equal(devices[0].model, "PHY110");
  assert.equal(devices[1].state, "unauthorized");
});

test("application labels merge with package names and always clean temporary device files", async () => {
  const calls = [];
  let failLabels = false;
  const native = {
    runtimePaths: async () => ({ toolsDir: "C:/tools with spaces" }),
    adbCommand: async args => {
      calls.push(args);
      if (args.includes("app_process")) {
        if (failLabels) throw Error("unsupported device");
        return output('{"com.example.app":"示例应用","com.empty.app":" ","com.bad.app":123}');
      }
      if (args.includes("packages")) return output(args.at(-1) === "-s" ? "package:android" : "package:android\npackage:com.example.app\npackage:com.empty.app");
      return output();
    },
  };
  const { adb } = load("adb", native);
  const apps = await adb.installedApps("emulator-5554");
  assert.deepEqual(apps.find(app => app.packageName === "com.example.app"), { packageName: "com.example.app", appName: "示例应用", system: false });
  assert.equal(apps.find(app => app.packageName === "com.empty.app").appName, "com.empty.app");
  assert.ok(calls.some(args => args.includes("C:/tools with spaces/app-info/app-labels.dex")));
  assert.deepEqual(calls.at(-1).slice(0, 5), ["-s", "emulator-5554", "shell", "rm", "-f"]);
  failLabels = true;
  assert.ok((await adb.installedApps("emulator-5554")).every(app => app.appName === app.packageName));
  assert.equal(calls.at(-1)[3], "rm");
});

test("install keeps file paths as one argument and checks both output streams", async () => {
  let call;
  const { adb } = load("adb", { adbCommand: async (args, options) => { call = { args, options }; return output("", "Success\n"); } });
  await adb.install("emulator-5554", "C:/APK files/示例.apk");
  assert.deepEqual(call.args, ["-s", "emulator-5554", "install", "-r", "C:/APK files/示例.apk"]);
  assert.equal(call.options.stream, true);
  const failure = load("adb", { adbCommand: async () => output("Failure [INSTALL_FAILED_TEST_ONLY]") });
  await assert.rejects(failure.adb.install("emulator-5554", "test.apk"), /未确认成功/);
  await assert.rejects(adb.uninstall("emulator-5554", "com.example;reboot"), /包名/);
});

test("OBB upload builds the destination in TS and rejects malformed filenames", async () => {
  const calls = [];
  const { adb } = load("adb", { adbCommand: async args => { calls.push(args); return output("1 file pushed"); } });
  await adb.pushObb("emulator-5554", "C:/Game files/main.42.com.example.app.obb");
  assert.deepEqual(calls[0], ["-s", "emulator-5554", "shell", "mkdir", "-p", "/sdcard/Android/obb/com.example.app"]);
  assert.equal(calls[1].at(-1), "/sdcard/Android/obb/com.example.app/main.42.com.example.app.obb");
  await assert.rejects(adb.pushObb("emulator-5554", "random.obb"), /OBB 文件名/);
});

function packageFixture() {
  const files = new Set(["original.apk", "decoded/AndroidManifest.xml", "java/Old.java"]);
  const calls = [], progress = [];
  let fail = false, omitOutput = false;
  const native = {
    runtimePaths: async () => ({ workDir: "C:/work space", toolsDir: "C:/tool space" }),
    getPackageInfo: async () => ({ decoded: files.has("decoded/AndroidManifest.xml"), hasJar: files.has("source.jar"), decompiled: files.has("java/New.java") }),
    packageFile: async (id, action, relative, destination) => {
      calls.push({ action, relative, destination });
      if (action === "isFile") return files.has(relative);
      const entries = [...files].filter(file => file === relative || file.startsWith(relative + "/"));
      if (action === "hasEntries") return entries.length > 0;
      if (action === "remove") { entries.forEach(file => files.delete(file)); return entries.length > 0; }
      if (action === "rename") { entries.forEach(file => { files.delete(file); files.add(destination + file.slice(relative.length)); }); return entries.length > 0; }
      throw Error(action);
    },
    javaCommand: async (args, options) => {
      calls.push({ args, options });
      if (fail) throw Error("tool failed");
      if (!omitOutput) files.add(options.source === "apktool" ? "decoded.tmp/AndroidManifest.xml" : options.source === "dex2jar" ? "source.tmp.jar" : "java.tmp/New.java");
      return output();
    },
  };
  return { native, files, calls, progress, api: load("packageCommands", native), fail: () => { fail = true; }, omit: () => { omitOutput = true; } };
}

test("decompile builds a missing JAR first and replaces outputs after tool success", async () => {
  const fixture = packageFixture();
  const info = await fixture.api.decompilePackage("sample-123", event => fixture.progress.push(event));
  assert.equal(info.hasJar, true);
  assert.equal(info.decompiled, true);
  assert.equal(fixture.files.has("java/Old.java"), false);
  const tools = fixture.calls.filter(call => call.args);
  assert.deepEqual(tools.map(call => call.options.source), ["dex2jar", "cfr"]);
  assert.equal(tools[0].options.cwd, "C:/tool space/dex-tools");
  assert.ok(tools[0].args.includes("C:/work space/sample-123/source.tmp.jar"));
  assert.ok(tools[1].args.includes("C:/tool space/cfr.jar"));
  assert.deepEqual(fixture.progress.map(event => [event.stage, event.state]), [["jar", "running"], ["jar", "done"], ["decompile", "running"], ["decompile", "done"]]);
});

test("tool failures retain old JAR/resources/source and settle task progress", async () => {
  for (const operation of ["decodePackage", "buildJar", "decompilePackage"]) {
    const fixture = packageFixture();
    fixture.files.add("source.jar");
    fixture.fail();
    await assert.rejects(fixture.api[operation]("sample-123", event => fixture.progress.push(event)), /tool failed/);
    assert.ok(fixture.files.has("source.jar"));
    assert.ok(fixture.files.has("decoded/AndroidManifest.xml"));
    assert.ok(fixture.files.has("java/Old.java"));
    assert.equal(fixture.progress.at(-1).state, "error");
  }
});

test("zero exit without expected outputs is an error and retains previous results", async () => {
  const fixture = packageFixture();
  fixture.omit();
  await assert.rejects(fixture.api.decodePackage("sample-123", () => {}), /没有生成 AndroidManifest/);
  assert.ok(fixture.files.has("decoded/AndroidManifest.xml"));
});

test("live device: TS builds ADB commands and reads localized app names", { skip: !process.env.MOBILETOOLBOX_TEST_DEVICE }, async () => {
  const toolsDir = path.resolve(__dirname, "../apps/app/src-tauri/resources/tools");
  const adbBinary = path.join(toolsDir, process.platform === "win32" ? "adb/adb.exe" : "adb/adb");
  const native = {
    runtimePaths: async () => ({ toolsDir }),
    adbCommand: async args => {
      const result = spawnSync(adbBinary, args, { encoding: "utf8", windowsHide: true });
      if (result.error) throw result.error;
      if (result.status !== 0) throw Error(result.stderr || result.stdout);
      return output(result.stdout, result.stderr);
    },
  };
  const { adb } = load("adb", native);
  assert.ok((await adb.devices()).some(device => device.id === process.env.MOBILETOOLBOX_TEST_DEVICE));
  const apps = await adb.installedApps(process.env.MOBILETOOLBOX_TEST_DEVICE);
  assert.ok(apps.length > 0);
  assert.ok(apps.some(app => app.appName !== app.packageName));
});
