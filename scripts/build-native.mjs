#!/usr/bin/env node
/** Build only the approved native dependency graph; reuse the installed bundler as a packaging tool. */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const formatsFor = platform => ({ win32: ['msi', 'nsis'], darwin: ['app', 'dmg'], linux: ['tar.gz'] })[platform];

export function parseOptions(args, platform = process.platform) {
  const result = { bundle: false, skipBuild: false, check: false, target: null, formats: [...(formatsFor(platform) ?? [])] };
  if (!result.formats.length) throw new Error('Native packaging supports Windows, macOS and Linux.');
  for (let index = 0; index < args.length; index++) {
    const argument = args[index];
    if (argument === '--bundle') result.bundle = true;
    else if (argument === '--skip-build') result.skipBuild = true;
    else if (argument === '--check') result.check = true;
    else if (argument === '--target') {
      result.target = args[++index];
      if (!result.target || !/^[a-z0-9_]+(?:-[a-z0-9_]+){2,}$/.test(result.target)) throw new Error('Use a valid Rust target triple.');
    } else if (argument === '--bundles') {
      result.bundle = true;
      const value = args[++index];
      if (!value) throw new Error('--bundles requires a comma-separated format list.');
      result.formats = [...new Set(value.split(','))];
      if (!result.formats.length || result.formats.some(format => !formatsFor(platform).includes(format))) {
        throw new Error(`Supported native bundle formats on this host: ${formatsFor(platform).join(', ')}.`);
      }
    } else throw new Error(`Unknown argument ${argument}. Use --check, --bundle, --bundles FORMATS, --skip-build or --target TARGET.`);
  }
  if (result.skipBuild && !result.bundle) throw new Error('--skip-build requires --bundle; use it only with a verified existing release binary.');
  return result;
}

export function assertNativeGraph(metadata) {
  const packages = new Map(metadata.packages.map(pkg => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  const application = metadata.packages.find(pkg => pkg.name === 'oxbox-native' && metadata.workspace_members.includes(pkg.id));
  if (!application) throw new Error('The native workspace package is missing.');
  const visited = new Set();
  const names = new Set();
  const queue = [application.id];
  while (queue.length) {
    const id = queue.pop();
    if (visited.has(id)) continue;
    visited.add(id);
    const pkg = packages.get(id);
    const node = nodes.get(id);
    if (!pkg || !node) throw new Error('The native dependency graph is incomplete.');
    if (/^(?:tauri(?:-|$)|wry$|webkit|webview2|javascriptcore|jsc-sys$)/i.test(pkg.name)) {
      // This small Windows notification adapter predates the migration and creates no browser.
      if (pkg.name !== 'tauri-winrt-notification') throw new Error(`Native runtime/build graph unexpectedly includes browser host package ${pkg.name}.`);
    }
    names.add(pkg.name);
    for (const dependency of node.deps) {
      if (dependency.dep_kinds.some(kind => kind.kind !== 'dev')) queue.push(dependency.pkg);
    }
  }
  return names;
}

export function nativeBuildEnvironment(environment, target) {
  const result = { ...environment };
  if (target.endsWith('-pc-windows-msvc')) {
    if (result.CARGO_ENCODED_RUSTFLAGS !== undefined) result.CARGO_ENCODED_RUSTFLAGS = `${result.CARGO_ENCODED_RUSTFLAGS}${result.CARGO_ENCODED_RUSTFLAGS ? '\x1f' : ''}-Ctarget-feature=+crt-static`;
    else result.RUSTFLAGS = `${result.RUSTFLAGS || ''} -Ctarget-feature=+crt-static`.trim();
  }
  return result;
}

export function bundleConfiguration(template, directory, version) {
  const config = structuredClone(template);
  if (config.mainBinaryName !== 'oxbox-native' || config.bundle.windows.webviewInstallMode.type !== 'skip'
    || config.bundle.createUpdaterArtifacts !== false || config.bundle.windows.minimumWebview2Version != null) {
    throw new Error('Native bundle must use oxbox-native with WebView installation and updater artifacts disabled.');
  }
  config.version = version;
  config.bundle.icon = config.bundle.icon.map(filename => path.resolve(directory, filename));
  config.bundle.resources = Object.fromEntries(Object.entries(config.bundle.resources).map(([filename, destination]) => [path.resolve(directory, filename), destination]));
  if (config.bundle.windows.wix?.fragmentPaths) config.bundle.windows.wix.fragmentPaths = config.bundle.windows.wix.fragmentPaths.map(filename => path.resolve(directory, filename));
  for (const field of ['installerIcon', 'uninstallerIcon']) {
    if (config.bundle.windows.nsis?.[field]) config.bundle.windows.nsis[field] = path.resolve(directory, config.bundle.windows.nsis[field]);
  }
  return config;
}
export function portableResources(resources) {
  return Object.entries(resources).filter(([, destination]) => destination !== 'oxbox-installed.json');
}
export function patchNsisUninstall(source) {
  const marker = '; Oxbox owned startup cleanup before executable removal';
  const runDelete = '    DeleteRegValue HKCU "Software\\Microsoft\\Windows\\CurrentVersion\\Run" "${PRODUCTNAME}"';
  const remove = '  Delete "$INSTDIR\\${MAINBINARYNAME}.exe"';
  const normalized = source.replaceAll('\r\n', '\n');
  if (normalized.includes(marker) && !normalized.includes(runDelete)) return source;
  const section = normalized.indexOf('Section Uninstall\n');
  if (section < 0 || normalized.split(runDelete).length !== 2 || normalized.indexOf(remove, section) < 0) {
    throw new Error('Unknown NSIS template shape; refusing to ship unreviewed startup deletion.');
  }
  const removal = normalized.indexOf(remove, section);
  const cleanup = [marker, '  ${If} $UpdateMode <> 1', '    ExecWait \'"$INSTDIR\\${MAINBINARYNAME}.exe" --uninstall-cleanup\' $0', '  ${EndIf}', ''].join('\n');
  const patched = normalized.slice(0, removal) + cleanup + normalized.slice(removal);
  return patched.replace(runDelete, '    ; The native cleanup command preserves any unrelated/user-repurposed Run value.');
}

export function patchNsisRunningCheck(source) {
  const normalized = source.replaceAll('\r\n', '\n');
  const marker = '; Oxbox requires explicit Quit; never force-stop owned services';
  const start = '!macro CheckIfAppIsRunning executablePath productName\n';
  const begin = normalized.indexOf(start);
  const end = normalized.indexOf('!macroend', begin);
  if (begin < 0 || end < 0 || normalized.indexOf(start, begin + start.length) >= 0) {
    throw new Error('Unknown NSIS running-app template; refusing to change lifecycle behavior.');
  }
  const replacement = `${start}${marker}
  !insertmacro RestartManager_StartSession $R0
  \${If} $R0 == ""
    SetErrorLevel 2
    Abort "Could not check running applications. Quit Oxbox and retry."
  \${EndIf}
  !insertmacro RestartManager_RegisterFile $R0 "\${executablePath}"
  \${If} $0 != 0
    !insertmacro RestartManager_EndSession $R0
    SetErrorLevel 2
    Abort "Could not check the installed executable. Quit Oxbox and retry."
  \${EndIf}
  System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
  StrCpy $R4 $0
  !insertmacro RestartManager_EndSession $R0
  \${If} $R4 = \${ERROR_MORE_DATA}
    IfSilent +2
      MessageBox MB_OK|MB_ICONSTOP "Quit Oxbox from its tray menu before continuing. Closing its window only hides it."
    SetErrorLevel 2
    Abort "Oxbox is running. Quit it from its tray menu before continuing."
  \${ElseIf} $R4 != 0
    SetErrorLevel 2
    Abort "Could not verify that Oxbox has stopped. Quit it and retry."
  \${EndIf}
!macroend`;
  const current = normalized.slice(begin, end + '!macroend'.length);
  if (current.includes(marker)) {
    if (current !== replacement) throw new Error('The generated NSIS lifecycle guard was modified; refusing to bypass review.');
    return normalized;
  }
  if (!current.includes('RmShutdown') || !current.includes('RmForceShutdown')) {
    throw new Error('Unknown NSIS running-app template; expected the reviewed Restart Manager force-shutdown path.');
  }
  return normalized.slice(0, begin) + replacement + normalized.slice(end + '!macroend'.length);
}

function rebuildNsis(out, target, config, environment) {
  const arch = target.startsWith('x86_64-') ? 'x64' : target.startsWith('aarch64-') ? 'arm64' : 'x86';
  const directory = path.join(out, 'nsis', arch);
  const filename = path.join(directory, 'installer.nsi');
  ordinary(filename);
  const source = fs.readFileSync(filename, 'utf8');
  const plugin = /^!define ADDITIONALPLUGINSPATH "([^"]+)"/m.exec(source)?.[1];
  const outputName = /^!define OUTFILE "([^"]+)"/m.exec(source)?.[1];
  if (!plugin || !outputName || path.basename(outputName) !== outputName) throw new Error('Unknown NSIS compiler/output configuration.');
  const compiler = path.resolve(plugin, '../../..', 'makensis.exe');
  ordinary(compiler);
  const utilities = path.join(directory, 'utils.nsh');
  ordinary(utilities);
  writeGenerated(utilities, patchNsisRunningCheck(fs.readFileSync(utilities, 'utf8')));
  writeGenerated(filename, patchNsisUninstall(source));
  const nsisEnvironment = { ...environment };
  delete nsisEnvironment.NSISDIR;
  delete nsisEnvironment.NSISCONFDIR;
  run(compiler, ['-INPUTCHARSET', 'UTF8', '-OUTPUTCHARSET', 'UTF8', '-V2', filename], nsisEnvironment, directory);
  const generated = path.join(directory, outputName);
  ordinary(generated);
  const installer = path.join(out, 'bundle/nsis', `${config.productName}_${config.version}_${arch}-setup.exe`);
  ordinary(installer);
  fs.copyFileSync(generated, installer);
  process.stdout.write('Rebuilt NSIS with exact-owned startup cleanup; upgrades preserve startup policy.\n');
}

export function windowsResourceDefinition(version, icon) {
  const parsed = /^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.exec(version);
  if (!parsed || parsed.slice(1).some(value => !Number.isSafeInteger(Number(value)) || Number(value) > 65535)) {
    throw new Error('Windows resource version needs three numeric components between 0 and 65535.');
  }
  const numeric = `${parsed.slice(1).map(Number).join(',')},0`;
  return `LANGUAGE 9,1
1 ICON ${JSON.stringify(icon)}
1 VERSIONINFO
FILEVERSION ${numeric}
PRODUCTVERSION ${numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x40004L
FILETYPE 1L
FILESUBTYPE 0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Oxbox contributors\\0"
      VALUE "FileDescription", "Oxbox native desktop toolkit\\0"
      VALUE "FileVersion", "${version}\\0"
      VALUE "InternalName", "oxbox-native\\0"
      VALUE "OriginalFilename", "oxbox-native.exe\\0"
      VALUE "ProductName", "Oxbox\\0"
      VALUE "ProductVersion", "${version}\\0"
      VALUE "LegalCopyright", "Copyright (c) Oxbox contributors\\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409,1200
  END
END
`;
}

function windowsResource(directory, target, version, environment) {
  const icon = path.join(root, 'src-tauri/icons/icon.ico');
  const source = windowsResourceDefinition(version, icon);
  const digest = crypto.createHash('sha256').update(source).update(fs.readFileSync(icon)).digest('hex').slice(0, 20);
  const stage = path.join(directory, 'native-resources', target);
  fs.mkdirSync(stage, { recursive: true });
  const resource = path.join(stage, `oxbox-${digest}.res`);
  if (fs.existsSync(resource)) {
    ordinary(resource);
    return resource;
  }
  const sdk = environment.WindowsSdkDir || path.join(environment['ProgramFiles(x86)'] || 'C:/Program Files (x86)', 'Windows Kits/10');
  const bin = path.join(sdk, 'bin');
  const versions = fs.readdirSync(bin, { withFileTypes: true })
    .filter(entry => entry.isDirectory() && /^\d+\.\d+\.\d+\.\d+$/.test(entry.name))
    .map(entry => entry.name).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));
  const compiler = versions.map(version => path.join(bin, version, 'x64/rc.exe')).find(filename => fs.existsSync(filename));
  if (!compiler) throw new Error('The existing Windows SDK resource compiler rc.exe is required for native EXE branding.');
  ordinary(compiler);
  const filename = path.join(stage, `oxbox-${digest}.rc`);
  if (fs.existsSync(filename)) ordinary(filename);
  // RC accepts UTF-16 source with BOM, including non-ASCII checkout/icon paths.
  fs.writeFileSync(filename, Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(source, 'utf16le')]));
  try {
    run(compiler, ['/nologo', '/fo', resource, filename], environment);
  } catch (error) {
    // This exact build output did not exist before RC ran; discard only a partial result.
    if (fs.existsSync(resource)) fs.unlinkSync(resource);
    throw error;
  }
  ordinary(resource);
  return resource;
}

function run(command, args, environment, cwd = root) {
  execFileSync(command, args, { cwd, env: environment, stdio: 'inherit' });
}
function json(command, args, environment) {
  return JSON.parse(execFileSync(command, args, { cwd: root, env: environment, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, stdio: ['ignore', 'pipe', 'inherit'] }));
}
function ordinary(filename) {
  const stat = fs.lstatSync(filename);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error(`Packaging input must be an ordinary file: ${path.basename(filename)}.`);
}
function writeGenerated(filename, text) {
  if (fs.existsSync(filename)) ordinary(filename);
  fs.writeFileSync(filename, text, { encoding: 'utf8', flag: 'w' });
}

export function main(args = process.argv.slice(2)) {
  const options = parseOptions(args);
  // Distribution must use exactly the user-approved, source-pinned backports.
  run(process.execPath, [path.join(root, 'scripts/verify-vendor.mjs')], process.env);
  const host = execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/^host:\s*(\S+)$/m)?.[1];
  if (!host) throw new Error('Could not determine the Rust host target.');
  const target = options.target || host;
  const platformTarget = process.platform === 'win32' ? /-windows-msvc$/ : process.platform === 'darwin' ? /-apple-darwin$/ : /-unknown-linux-gnu$/;
  if (!platformTarget.test(target)) throw new Error('Build/package on the matching OS; supported Linux bundles require a GNU target.');
  const environment = nativeBuildEnvironment({
    ...process.env,
    CARGO_TARGET_DIR: path.resolve(process.env.CARGO_TARGET_DIR || path.join(root, 'target/native-dist')),
  }, target);
  const metadata = json('cargo', ['metadata', '--locked', '--offline', '--format-version', '1', '--filter-platform', target], environment);
  const graph = assertNativeGraph(metadata);
  const application = metadata.packages.find(pkg => pkg.name === 'oxbox-native' && metadata.workspace_members.includes(pkg.id));
  if (process.env.OXBOX_RELEASE_TAG && process.env.OXBOX_RELEASE_TAG !== `v${application.version}`) {
    throw new Error('The release tag must match the native Cargo package version.');
  }
  const template = JSON.parse(fs.readFileSync(path.join(root, 'packaging/native/tauri.conf.json'), 'utf8'));
  const config = bundleConfiguration(template, root, application.version);
  for (const filename of [...config.bundle.icon, ...Object.keys(config.bundle.resources)]) ordinary(filename);
  const out = path.join(metadata.target_directory, target, 'release');
  const executable = path.join(out, `oxbox-native${process.platform === 'win32' ? '.exe' : ''}`);
  if (options.check) {
    process.stdout.write(`Verified native graph: ${graph.size} package names, target ${target}; no Tauri/WebView host. Packaging: ${options.formats.join(', ')}.\n`);
    return;
  }
  if (!options.skipBuild) {
    // Explicit target keeps static-CRT flags off host proc-macro/build-script crates.
    const resource = process.platform === 'win32' ? windowsResource(metadata.target_directory, target, application.version, environment) : null;
    run('cargo', [resource ? 'rustc' : 'build', '-p', 'oxbox-native', '--release', '--locked', '--target', target,
      ...(resource ? ['--', `-Clink-arg=${resource}`] : [])], environment);
  }
  ordinary(executable);
  process.stdout.write(`Native binary: ${executable}\n`);
  if (!options.bundle) return;

  if (process.platform === 'linux') {
    // Tauri CLI injects WebKit dependencies into Linux packages unconditionally.
    // A portable native archive records the real runtime requirements instead.
    const name = `oxbox-${application.version}-${target}`;
    // A fresh staging directory cannot inherit an installed-only marker or stale
    // payload from a preceding build of the same version.
    const stagingRoot = fs.mkdtempSync(path.join(metadata.target_directory, 'native-distribution-'));
    const stage = path.join(stagingRoot, name);
    fs.mkdirSync(stage, { recursive: true });
    for (const [source, destination] of [[executable, 'oxbox-native'], ...portableResources(config.bundle.resources)]) {
      if (path.basename(destination) !== destination) throw new Error('Portable resource destinations must be plain filenames.');
      const output = path.join(stage, destination);
      if (fs.existsSync(output)) ordinary(output);
      fs.copyFileSync(source, output);
    }
    fs.chmodSync(path.join(stage, 'oxbox-native'), 0o755);
    const archive = path.join(out, `${name}.tar.gz`);
    if (fs.existsSync(archive)) ordinary(archive);
    run('tar', ['-czf', archive, '-C', path.dirname(stage), name], environment);
    process.stdout.write(`Native portable archive: ${archive}\n`);
    return;
  }

  // The bundler only needs package metadata and an existing executable. Isolating
  // this metadata-only project keeps the legacy renderer and native manifest intact.
  const stage = path.join(metadata.target_directory, 'native-bundle-project', target);
  fs.mkdirSync(stage, { recursive: true });
  const quote = value => JSON.stringify(value);
  writeGenerated(path.join(stage, 'Cargo.toml'), `[package]\nname = "oxbox-native"\nversion = ${quote(application.version)}\nedition = "2024"\nauthors = ["Oxbox contributors"]\nlicense = "MIT OR Apache-2.0"\ndescription = "Oxbox native desktop toolkit"\ndefault-run = "oxbox-native"\n\n[[bin]]\nname = "oxbox-native"\npath = ${quote(path.join(path.dirname(application.manifest_path), 'src/main.rs'))}\n\n[workspace]\n`);
  writeGenerated(path.join(stage, 'tauri.conf.json'), `${JSON.stringify(config, null, 2)}\n`);
  const cli = path.join(root, 'node_modules/@tauri-apps/cli/tauri.js');
  ordinary(cli);
  run(process.execPath, [cli, 'bundle', '--ci', '--no-sign', '--no-binary-patching', '--bundles', options.formats.join(','), '--target', target], {
    ...environment, CARGO_TARGET_DIR: metadata.target_directory, TAURI_APP_PATH: stage, TAURI_FRONTEND_PATH: root,
  });
  if (process.platform === 'win32' && options.formats.includes('nsis')) rebuildNsis(out, target, config, environment);
  process.stdout.write(`Native bundles: ${path.join(out, 'bundle')}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
