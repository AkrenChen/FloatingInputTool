import fs from 'node:fs'
import path from 'node:path'
import os from 'node:os'
import { execFileSync } from 'node:child_process'

const root = process.cwd()
const source = fs.readFileSync(path.join(root, 'tauri-app/src-tauri/src/lib.rs'), 'utf8')
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'floating-review-'))
const formatting = source.slice(source.indexOf('fn sanitize_note_template('), source.indexOf('fn prepend_to_file('))
const geometry = source.slice(source.indexOf('fn compute_docked_bounds<'), source.indexOf('fn restore_main_window_to_normal<'))
const harness = `
#![allow(dead_code)]
use std::marker::PhantomData;
const DEFAULT_NOTE_TEMPLATE: &str = "> [!fleeting]+ {{timestamp}}\\n>\\n{{text.callout}}";
fn current_timestamp_string() -> String { "2026-10-27 10:00".into() }
trait Runtime {}
struct TestRuntime;
impl Runtime for TestRuntime {}
#[derive(Clone, Copy, Debug)]
enum DockSide { Left, Right }
#[derive(Clone, Copy, Debug)]
struct StoredWindowBounds { x: i32, y: i32, width: u32, height: u32 }
struct Position { x: i32, y: i32 }
struct Size { width: u32, height: u32 }
struct WorkArea { position: Position, size: Size }
struct Monitor;
impl Monitor {
  fn work_area(&self) -> WorkArea {
    WorkArea { position: Position { x: 0, y: 0 }, size: Size { width: 1920, height: 1080 } }
  }
}
struct WebviewWindow<R: Runtime>(PhantomData<R>);
impl<R: Runtime> WebviewWindow<R> {
  fn current_monitor(&self) -> Result<Option<Monitor>, String> { Ok(Some(Monitor)) }
}
struct AppConfig { edge_snap_threshold_px: u32, visible_handle_width_px: u32, hotzone_width_px: u32 }
${formatting}
${geometry}
fn main() {
  let input = "literal {{text}}";
  let output = format_note_block(input, "{{text.callout}}");
  assert_eq!(output, "> literal literal {{text}}\\r\\n\\r\\n");
  println!("CONFIRMED template rewrites note text: {:?}", output);
  let window = WebviewWindow::<TestRuntime>(PhantomData);
  let config = AppConfig { edge_snap_threshold_px: 36, visible_handle_width_px: 22, hotzone_width_px: 36 };
  let normal = StoredWindowBounds { x: 0, y: 100, width: 560, height: 360 };
  let (_, first_dock) = compute_docked_bounds(&window, &normal, &config).unwrap().unwrap();
  let (_, second_dock) = compute_docked_bounds(&window, &first_dock, &config).unwrap().unwrap();
  assert_eq!(first_dock.x, -524);
  assert_eq!(second_dock.x, -524);
  println!("GEOMETRY repeated Hide stores current x={} as normal x; Capture applies it directly", first_dock.x);
  let visible_physical_width = second_dock.x + second_dock.width as i32;
  assert_eq!(visible_physical_width, 36);
  println!("GEOMETRY visible width={} physical pixels => {} CSS pixels at 150%, {} at 200%", visible_physical_width, visible_physical_width as f64 / 1.5, visible_physical_width as f64 / 2.0);
}
`
const inputPath = path.join(temporary, 'probe.rs')
const binaryPath = path.join(temporary, 'probe.exe')
fs.writeFileSync(inputPath, harness)
execFileSync('rustc', ['--edition=2021', inputPath, '-o', binaryPath], { stdio: 'inherit' })
process.stdout.write(execFileSync(binaryPath, { encoding: 'utf8' }))
const config = JSON.parse(fs.readFileSync(path.join(root, 'tauri-app/src-tauri/tauri.conf.json'), 'utf8'))
const schema = JSON.parse(fs.readFileSync(path.join(root, 'tauri-app/node_modules/@tauri-apps/cli/config.schema.json'), 'utf8'))
console.log('STARTUP visible:', config.app.windows[0].visible ?? schema.definitions.WindowConfig.properties.visible.default)
const acl = JSON.parse(fs.readFileSync(path.join(root, 'tauri-app/src-tauri/gen/schemas/acl-manifests.json'), 'utf8'))
const capability = JSON.parse(fs.readFileSync(path.join(root, 'tauri-app/src-tauri/capabilities/default.json'), 'utf8'))
console.log('SETTINGS setFocus permission:', capability.permissions.includes('core:window:allow-set-focus') || acl['core:window'].default_permission.permissions.includes('allow-set-focus'))
console.log('Probe artifacts:', temporary)
