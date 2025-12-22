use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, DropDown, Label, Orientation, Scale, StringList,
};
use gdk4::RGBA;
use std::cell::{Cell, RefCell};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use glob::glob;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::thread;

const KB_BACKLIGHT_PATTERN: &str = "/sys/class/leds/rgb:kbd_backlight*";
const LIGHTBAR_PATH: &str = "/sys/class/leds/rgb:lightbar";

fn should_write_all_keyboard_paths() -> bool {
    match std::env::var("KBD_WRITE_ALL") {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => false,
    }
}

fn should_write_primary_only_keyboard_path() -> bool {
    match std::env::var("KBD_WRITE_PRIMARY_ONLY") {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => false,
    }
}

fn keyboard_write_paths(all_paths: &[PathBuf], primary_path: &PathBuf) -> Vec<PathBuf> {
    // Many devices expose one LED per-key (or per-zone) as separate sysfs entries
    // (e.g. `rgb:kbd_backlight_1`, `rgb:kbd_backlight_2`, ...). In that case,
    // writing only the "primary" path often doesn't affect the visible backlight.
    // Default to writing all paths to match user expectations.
    if all_paths.len() <= 1
        || should_write_all_keyboard_paths()
        || !should_write_primary_only_keyboard_path()
    {
        return all_paths.to_vec();
    }
    vec![primary_path.clone()]
}

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn unpack_rgb(v: u32) -> (u8, u8, u8) {
    (((v >> 16) & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8)
}

fn notify_coalescer(tx: &mpsc::SyncSender<()>) {
    let _ = tx.try_send(());
}

fn rgba_to_rgb8(rgba: &RGBA) -> (u8, u8, u8) {
    let r = (rgba.red().clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgba.green().clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgba.blue().clamp(0.0, 1.0) * 255.0).round() as u8;
    (r, g, b)
}

fn persist_color_state(
    kb_color: u32,
    kb_brightness: i32,
    lb_color: Option<u32>,
    lb_brightness: Option<i32>,
) {
    let Some(home) = env::var("HOME").ok() else { return; };
    let path = PathBuf::from(home).join(".rusty-kb").join("colors.txt");
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Error creating state dir {:?}: {}", parent, e);
            return;
        }
    }
    let (kr, kg, kbv) = unpack_rgb(kb_color);
    let lb_line = if let Some(lb) = lb_color {
        let (r, g, b) = unpack_rgb(lb);
        format!("{} {} {} {}", r, g, b, lb_brightness.unwrap_or(0))
    } else {
        format!("0 0 0 {}", lb_brightness.unwrap_or(0))
    };
    let content = format!("{} {} {} {}\n{}\n", kr, kg, kbv, kb_brightness, lb_line);
    if let Err(e) = fs::write(&path, content) {
        eprintln!("Error writing color state {:?}: {}", path, e);
    }
}

fn dropdown_for_colors(
    presets: &[(&'static str, RGBA)],
    initial_rgb: (u8, u8, u8),
    on_select: impl Fn(RGBA) + 'static,
) -> DropDown {
    let names: Vec<&str> = presets.iter().map(|(name, _)| *name).collect();
    let list = StringList::new(&names);
    let dropdown = DropDown::builder().model(&list).build();

    let initial_index = presets
        .iter()
        .position(|(_, rgba)| rgba_to_rgb8(rgba) == initial_rgb)
        .unwrap_or(0);
    dropdown.set_selected(initial_index as u32);

    let presets_for_cb: Vec<RGBA> = presets.iter().map(|(_, rgba)| *rgba).collect();
    dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected() as usize;
        if let Some(rgba) = presets_for_cb.get(idx) {
            on_select(*rgba);
        }
    });

    dropdown
}

fn find_kb_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = glob(KB_BACKLIGHT_PATTERN) {
        for e in entries.flatten() {
            println!("Found keyboard backlight path: {:?}", e);
            out.push(e);
        }
    }
    println!("Total keyboard backlight paths found: {}", out.len());
    out
}

fn pick_primary(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }
    for p in paths {
        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
            if name == "rgb:kbd_backlight" {
                return Some(p.clone());
            }
        }
    }
    Some(paths[0].clone())
}

fn write_color(path: &PathBuf, r: u8, g: u8, b: u8) {
    let color_path = path.join("multi_intensity");
    let content = format!("{} {} {}\n", r, g, b);
    if let Err(e) = fs::write(&color_path, content) {
        eprintln!("Error: {}", e);
    }
}

fn write_brightness(path: &PathBuf, val: i32) {
    let brightness_path = path.join("brightness");
    if let Err(e) = fs::write(&brightness_path, format!("{}\n", val)) {
        eprintln!("Error: {}", e);
    }
}

fn open_writers(paths: &[PathBuf], leaf: &str) -> Vec<File> {
    let mut out = Vec::new();
    for p in paths {
        match OpenOptions::new().write(true).open(p.join(leaf)) {
            Ok(f) => out.push(f),
            Err(e) => eprintln!("Error opening {:?}/{}: {}", p, leaf, e),
        }
    }
    out
}

fn write_all_files(files: &mut [File], content: &[u8]) {
    for f in files {
        if let Err(e) = f
            .seek(SeekFrom::Start(0))
            .and_then(|_| f.write_all(content))
            .and_then(|_| f.flush())
        {
            eprintln!("Error writing sysfs value: {}", e);
        }
    }
}

fn write_color_all(paths: &[PathBuf], r: u8, g: u8, b: u8) {
    for p in paths {
        write_color(p, r, g, b);
    }
}

fn write_brightness_all(paths: &[PathBuf], val: i32) {
    for p in paths {
        write_brightness(p, val);
    }
}

fn read_color(path: &PathBuf) -> Option<(u8, u8, u8)> {
    let color_path = path.join("multi_intensity");
    if let Ok(content) = fs::read_to_string(color_path) {
        let parts: Vec<u8> = content
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() == 3 {
            return Some((parts[0], parts[1], parts[2]));
        }
    }
    None
}

fn read_brightness(path: &PathBuf) -> Option<i32> {
    let brightness_path = path.join("brightness");
    if let Ok(content) = fs::read_to_string(brightness_path) {
        return content.trim().parse().ok();
    }
    None
}

// Coalescing worker: only apply the most recent value received (no sender-side backlog)
fn spawn_kb_color_worker(paths: Vec<PathBuf>) -> (Arc<AtomicU32>, mpsc::SyncSender<()>) {
    let latest = Arc::new(AtomicU32::new(0));
    let latest_for_thread = Arc::clone(&latest);
    let (tx, rx) = mpsc::sync_channel::<()>(1);
    thread::spawn(move || {
        let mut last_applied: Option<u32> = None;
        let mut color_files = open_writers(&paths, "multi_intensity");
        while rx.recv().is_ok() {
            while rx.try_recv().is_ok() {}
            let v = latest_for_thread.load(Ordering::Relaxed);
            if last_applied == Some(v) {
                continue;
            }
            let (r, g, b) = unpack_rgb(v);
            if !color_files.is_empty() {
                let content = format!("{} {} {}\n", r, g, b);
                write_all_files(&mut color_files, content.as_bytes());
            } else {
                write_color_all(&paths, r, g, b);
            }
            last_applied = Some(v);
        }
    });
    (latest, tx)
}

fn spawn_kb_brightness_worker(paths: Vec<PathBuf>) -> (Arc<AtomicI32>, mpsc::SyncSender<()>) {
    let latest = Arc::new(AtomicI32::new(0));
    let latest_for_thread = Arc::clone(&latest);
    let (tx, rx) = mpsc::sync_channel::<()>(1);
    thread::spawn(move || {
        let mut last_applied: Option<i32> = None;
        let mut brightness_files = open_writers(&paths, "brightness");
        while rx.recv().is_ok() {
            while rx.try_recv().is_ok() {}
            let v = latest_for_thread.load(Ordering::Relaxed);
            if last_applied == Some(v) {
                continue;
            }
            if !brightness_files.is_empty() {
                let content = format!("{}\n", v);
                write_all_files(&mut brightness_files, content.as_bytes());
            } else {
                write_brightness_all(&paths, v);
            }
            last_applied = Some(v);
        }
    });
    (latest, tx)
}

fn spawn_lb_color_worker(path: PathBuf) -> (Arc<AtomicU32>, mpsc::SyncSender<()>) {
    let latest = Arc::new(AtomicU32::new(0));
    let latest_for_thread = Arc::clone(&latest);
    let (tx, rx) = mpsc::sync_channel::<()>(1);
    thread::spawn(move || {
        let mut last_applied: Option<u32> = None;
        while rx.recv().is_ok() {
            while rx.try_recv().is_ok() {}
            let v = latest_for_thread.load(Ordering::Relaxed);
            if last_applied == Some(v) {
                continue;
            }
            let (r, g, b) = unpack_rgb(v);
            write_color(&path, r, g, b);
            last_applied = Some(v);
        }
    });
    (latest, tx)
}

fn spawn_lb_brightness_worker(path: PathBuf) -> (Arc<AtomicI32>, mpsc::SyncSender<()>) {
    let latest = Arc::new(AtomicI32::new(0));
    let latest_for_thread = Arc::clone(&latest);
    let (tx, rx) = mpsc::sync_channel::<()>(1);
    thread::spawn(move || {
        let mut last_applied: Option<i32> = None;
        while rx.recv().is_ok() {
            while rx.try_recv().is_ok() {}
            let v = latest_for_thread.load(Ordering::Relaxed);
            if last_applied == Some(v) {
                continue;
            }
            write_brightness(&path, v);
            last_applied = Some(v);
        }
    });
    (latest, tx)
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.keyboard_controller")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Keyboard & Lightbar Controller")
            .default_width(400)
            .default_height(300)
            .build();

        let container = Box::new(Orientation::Vertical, 18);
        container.set_margin_top(24);
        container.set_margin_bottom(24);
        container.set_margin_start(24);
        container.set_margin_end(24);

        let shared_lb_color = Arc::new(AtomicU32::new(0));
        let shared_lb_brightness = Arc::new(AtomicI32::new(0));
        let lb_available = Rc::new(Cell::new(false));
        let kb_color_shared: Rc<RefCell<Option<Arc<AtomicU32>>>> = Rc::new(RefCell::new(None));
        let kb_brightness_shared: Rc<RefCell<Option<Arc<AtomicI32>>>> = Rc::new(RefCell::new(None));

        // Keyboard Section
        let kb_paths = find_kb_paths();
        if let Some(primary_path) = pick_primary(&kb_paths) {
            let kb_write_paths = keyboard_write_paths(&kb_paths, &primary_path);
            println!(
                "Keyboard writes will target {} path(s) (set KBD_WRITE_PRIMARY_ONLY=1 to write only {:?}).",
                kb_write_paths.len(),
                &primary_path
            );
            let section = Box::new(Orientation::Vertical, 8);
            
            let kb_label = Label::builder()
                .label("Keyboard Backlight")
                .halign(gtk4::Align::Start)
                .build();
            kb_label.add_css_class("title-4");
            section.append(&kb_label);

            let color_box = Box::new(Orientation::Horizontal, 10);
            color_box.append(&Label::new(Some("Color:")));

            // Initialize color from current hardware state
            let initial_kb_color = read_color(&primary_path).unwrap_or((255, 255, 255));

            // Updates via coalescing worker (applied on SetColor)
            let (latest_kb_color, tx_kb_color) = spawn_kb_color_worker(kb_write_paths.clone());
            // Sync initial hardware color across all per-key LEDs so the UI state matches
            // what will happen when you start changing colors.
            latest_kb_color.store(
                pack_rgb(initial_kb_color.0, initial_kb_color.1, initial_kb_color.2),
                Ordering::Relaxed,
            );
            notify_coalescer(&tx_kb_color);
            *kb_color_shared.borrow_mut() = Some(Arc::clone(&latest_kb_color));

            let preset_colors = [
                ("Red", RGBA::new(1.0, 0.0, 0.0, 1.0)),
                ("Blue", RGBA::new(0.0, 0.0, 1.0, 1.0)),
                ("Green", RGBA::new(0.0, 1.0, 0.0, 1.0)),
                ("Pink", RGBA::new(1.0, 0.41, 0.71, 1.0)),
                ("Orange", RGBA::new(1.0, 0.5, 0.0, 1.0)),
                ("Light Blue", RGBA::new(0.0, 0.6, 1.0, 1.0)),
            ];
            let latest_kb_color_for_dropdown = Arc::clone(&latest_kb_color);
            let tx_kb_color_for_dropdown = tx_kb_color.clone();
            let lb_color_for_dropdown = Arc::clone(&shared_lb_color);
            let lb_available_for_dropdown = lb_available.clone();
            let kb_brightness_for_dropdown = kb_brightness_shared.clone();
            let lb_brightness_for_dropdown = Arc::clone(&shared_lb_brightness);
            let dropdown = dropdown_for_colors(&preset_colors, initial_kb_color, move |rgba| {
                let (r, g, b) = rgba_to_rgb8(&rgba);
                latest_kb_color_for_dropdown.store(pack_rgb(r, g, b), Ordering::Relaxed);
                notify_coalescer(&tx_kb_color_for_dropdown);
                let lb_state = if lb_available_for_dropdown.get() {
                    Some(lb_color_for_dropdown.load(Ordering::Relaxed))
                } else {
                    None
                };
                let kb_brightness_val = kb_brightness_for_dropdown
                    .borrow()
                    .as_ref()
                    .map(|b: &Arc<AtomicI32>| b.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let lb_brightness_val = if lb_available_for_dropdown.get() {
                    Some(lb_brightness_for_dropdown.load(Ordering::Relaxed))
                } else {
                    None
                };
                persist_color_state(
                    latest_kb_color_for_dropdown.load(Ordering::Relaxed),
                    kb_brightness_val,
                    lb_state,
                    lb_brightness_val,
                );
            });

            color_box.append(&dropdown);
            section.append(&color_box);

            let bright_box = Box::new(Orientation::Vertical, 4);
            bright_box.append(&Label::builder().label("Brightness (0-50)").halign(gtk4::Align::Start).build());
            let kb_bright_scale = Scale::with_range(Orientation::Horizontal, 0.0, 50.0, 1.0);
            // Initialize brightness from current hardware state
            let initial_kb_brightness = read_brightness(&primary_path);
            if let Some(val) = initial_kb_brightness {
                kb_bright_scale.set_value(val as f64);
            }
            kb_bright_scale.set_draw_value(true);
            kb_bright_scale.set_digits(0);
            // Make scroll/keyboard adjustments feel snappier while keeping drag smooth.
            kb_bright_scale.set_increments(5.0, 10.0);
            let (latest_kb_bright, tx_kb_bright) = spawn_kb_brightness_worker(kb_write_paths.clone());
            *kb_brightness_shared.borrow_mut() = Some(Arc::clone(&latest_kb_bright));
            let latest_kb_bright_for_cb = Arc::clone(&latest_kb_bright);
            let tx_kb_bright_for_cb = tx_kb_bright.clone();
            let kb_color_for_brightness = kb_color_shared.clone();
            let lb_available_for_brightness = lb_available.clone();
            let lb_color_for_brightness = Arc::clone(&shared_lb_color);
            let lb_brightness_for_brightness = Arc::clone(&shared_lb_brightness);
            kb_bright_scale.connect_value_changed(move |scale| {
                let val = scale.value() as i32;
                latest_kb_bright_for_cb.store(val, Ordering::Relaxed);
                notify_coalescer(&tx_kb_bright_for_cb);
                let kb_color_val = kb_color_for_brightness
                    .borrow()
                    .as_ref()
                    .map(|c: &Arc<AtomicU32>| c.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let lb_color_val = if lb_available_for_brightness.get() {
                    Some(lb_color_for_brightness.load(Ordering::Relaxed))
                } else {
                    None
                };
                let lb_brightness_val = if lb_available_for_brightness.get() {
                    Some(lb_brightness_for_brightness.load(Ordering::Relaxed))
                } else {
                    None
                };
                persist_color_state(kb_color_val, val, lb_color_val, lb_brightness_val);
            });
            // Sync initial brightness across all per-key LEDs. Many devices expose
            // per-key LEDs with independent brightness values; if most are 0,
            // changing color appears to affect only a single key.
            if let Some(val) = initial_kb_brightness {
                latest_kb_bright.store(val, Ordering::Relaxed);
                notify_coalescer(&tx_kb_bright);
            }
            bright_box.append(&kb_bright_scale);
            section.append(&bright_box);
            
            container.append(&section);
        } else {
            container.append(&Label::new(Some("Keyboard Backlight not found")));
        }

        // Separator
        container.append(&gtk4::Separator::new(Orientation::Horizontal));

        // Lightbar Section
        let lb_path = PathBuf::from(LIGHTBAR_PATH);
        if lb_path.exists() {
            lb_available.set(true);
            let section = Box::new(Orientation::Vertical, 8);

            let lb_label = Label::builder()
                .label("Lightbar")
                .halign(gtk4::Align::Start)
                .build();
            lb_label.add_css_class("title-4");
            section.append(&lb_label);

            let color_box = Box::new(Orientation::Horizontal, 10);
            color_box.append(&Label::new(Some("Color:")));
            // Initialize color from current hardware state
            let initial_lb_color = read_color(&lb_path).unwrap_or((255, 255, 255));
            let (latest_lb_color, tx_lb_color) = spawn_lb_color_worker(lb_path.clone());

            // Sync initial color so state is consistent when applying further updates.
            latest_lb_color.store(
                pack_rgb(initial_lb_color.0, initial_lb_color.1, initial_lb_color.2),
                Ordering::Relaxed,
            );
            notify_coalescer(&tx_lb_color);
            shared_lb_color.store(
                pack_rgb(initial_lb_color.0, initial_lb_color.1, initial_lb_color.2),
                Ordering::Relaxed,
            );

            let preset_colors = [
                ("Red", RGBA::new(1.0, 0.0, 0.0, 1.0)),
                ("Blue", RGBA::new(0.0, 0.0, 1.0, 1.0)),
                ("Green", RGBA::new(0.0, 1.0, 0.0, 1.0)),
                ("Pink", RGBA::new(1.0, 0.41, 0.71, 1.0)),
                ("Orange", RGBA::new(1.0, 0.5, 0.0, 1.0)),
                ("Light Blue", RGBA::new(0.0, 0.6, 1.0, 1.0)),
            ];
            let latest_lb_color_for_dropdown = Arc::clone(&latest_lb_color);
            let tx_lb_color_for_dropdown = tx_lb_color.clone();
            let shared_lb_color_for_dropdown = Arc::clone(&shared_lb_color);
            let kb_color_for_dropdown = kb_color_shared.clone();
            let kb_brightness_for_dropdown = kb_brightness_shared.clone();
            let lb_brightness_for_dropdown = Arc::clone(&shared_lb_brightness);
            let dropdown = dropdown_for_colors(&preset_colors, initial_lb_color, move |rgba| {
                let (r, g, b) = rgba_to_rgb8(&rgba);
                latest_lb_color_for_dropdown.store(pack_rgb(r, g, b), Ordering::Relaxed);
                notify_coalescer(&tx_lb_color_for_dropdown);
                shared_lb_color_for_dropdown.store(pack_rgb(r, g, b), Ordering::Relaxed);
                let kb_state = kb_color_for_dropdown
                    .borrow()
                    .as_ref()
                    .map(|c: &Arc<AtomicU32>| c.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let kb_brightness_val = kb_brightness_for_dropdown
                    .borrow()
                    .as_ref()
                    .map(|b: &Arc<AtomicI32>| b.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let lb_brightness_val = lb_brightness_for_dropdown.load(Ordering::Relaxed);
                persist_color_state(
                    kb_state,
                    kb_brightness_val,
                    Some(latest_lb_color_for_dropdown.load(Ordering::Relaxed)),
                    Some(lb_brightness_val),
                );
            });

            color_box.append(&dropdown);
            section.append(&color_box);

            let bright_box = Box::new(Orientation::Vertical, 4);
            bright_box.append(&Label::builder().label("Brightness (0-100)").halign(gtk4::Align::Start).build());
            let lb_bright_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 5.0);
            // Initialize brightness from current hardware state
            if let Some(val) = read_brightness(&lb_path) {
                lb_bright_scale.set_value(val as f64);
            }
            lb_bright_scale.set_draw_value(true);
            lb_bright_scale.set_digits(0);
            lb_bright_scale.set_increments(10.0, 25.0);
            let (latest_lb_bright, tx_lb_bright) = spawn_lb_brightness_worker(lb_path.clone());
            shared_lb_brightness.store(
                read_brightness(&lb_path).unwrap_or(0),
                Ordering::Relaxed,
            );
            let shared_lb_brightness_for_cb = Arc::clone(&shared_lb_brightness);
            let kb_color_for_lb_brightness = kb_color_shared.clone();
            let kb_brightness_for_lb_brightness = kb_brightness_shared.clone();
            lb_bright_scale.connect_value_changed(move |scale| {
                let val = scale.value() as i32;
                latest_lb_bright.store(val, Ordering::Relaxed);
                notify_coalescer(&tx_lb_bright);
                shared_lb_brightness_for_cb.store(val, Ordering::Relaxed);
                let kb_color_val = kb_color_for_lb_brightness
                    .borrow()
                    .as_ref()
                    .map(|c: &Arc<AtomicU32>| c.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let kb_brightness_val = kb_brightness_for_lb_brightness
                    .borrow()
                    .as_ref()
                    .map(|b: &Arc<AtomicI32>| b.load(Ordering::Relaxed))
                    .unwrap_or(0);
                persist_color_state(
                    kb_color_val,
                    kb_brightness_val,
                    Some(shared_lb_color.load(Ordering::Relaxed)),
                    Some(val),
                );
            });
            bright_box.append(&lb_bright_scale);
            section.append(&bright_box);

            container.append(&section);
        } else {
            container.append(&Label::new(Some("Lightbar not found")));
        }

        // Exit Button
        let exit_box = Box::new(Orientation::Horizontal, 0);
        exit_box.set_halign(gtk4::Align::End);
        let exit_btn = Button::with_label("Exit");
        let window_weak = window.downgrade();
        exit_btn.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.close();
            }
        });
        exit_box.append(&exit_btn);
        container.append(&exit_box);

        window.set_child(Some(&container));
        window.present();
    });

    app.run();
}
