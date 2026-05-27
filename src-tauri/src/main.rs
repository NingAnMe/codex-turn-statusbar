mod codex_ipc;

use codex_turn_status_core::{
    install_notify_helper, merge_display_status, DisplayStatus, MenuBarPresentation, MenuBarTint,
    MenuContentKey, MenuRefreshState, StatusState, StatusStore, UnreadTracker,
};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Runtime, Wry};

const MENU_OPEN_CODEX: &str = "open-codex";
const MENU_MARK_HANDLED: &str = "mark-handled";
const MENU_REFRESH: &str = "refresh";
const MENU_QUIT: &str = "quit";

struct TrayAppState {
    store: StatusStore,
    unread: Arc<Mutex<UnreadTracker>>,
    tray: TrayIcon<Wry>,
    menu_refresh: Mutex<MenuRefreshState>,
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let store = StatusStore::default()?;
            install_packaged_notify_helper(&store);
            let unread = Arc::new(Mutex::new(UnreadTracker::new()));
            codex_ipc::start_unread_monitor(store.clone(), unread.clone());

            let notify_status = store.load_display_status();
            let can_mark_handled = notify_status.state == StatusState::NeedsAttention;
            let status = merge_status_parts(notify_status, &unread);
            let mut menu_refresh = MenuRefreshState::new();
            let _ =
                menu_refresh.should_rebuild(MenuContentKey::from_status(&status, can_mark_handled));
            let tray = TrayIconBuilder::with_id("codex-turn-status")
                .icon(icon_for_status(&status))
                .tooltip(MenuBarPresentation::from_status(&status).tooltip)
                .menu(&build_menu(app.handle(), &status, can_mark_handled)?)
                .show_menu_on_left_click(true)
                .on_menu_event(handle_menu_event)
                .build(app)?;

            let state = Arc::new(TrayAppState {
                store,
                unread,
                tray,
                menu_refresh: Mutex::new(menu_refresh),
            });
            app.manage(state.clone());
            update_tray(app.handle(), &state);

            let app_handle = app.handle().clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(1));
                let _ = app_handle.run_on_main_thread({
                    let app_handle = app_handle.clone();
                    move || {
                        if let Some(state) = app_handle.try_state::<Arc<TrayAppState>>() {
                            update_tray(&app_handle, &state);
                        }
                    }
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Codex Turn Status Bar");
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    let Some(state) = app.try_state::<Arc<TrayAppState>>() else {
        return;
    };

    match event.id().as_ref() {
        MENU_OPEN_CODEX => {
            open_codex();
        }
        MENU_MARK_HANDLED => mark_handled_if_needed(app, &state),
        MENU_REFRESH => force_update_tray_menu(app, &state),
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}

fn update_tray<R: Runtime>(app: &AppHandle<R>, state: &TrayAppState) {
    let notify_status = state.store.load_display_status();
    let can_mark_handled = notify_status.state == StatusState::NeedsAttention;
    let status = merge_status_parts(notify_status, &state.unread);

    let presentation = MenuBarPresentation::from_status(&status);
    let _ = state.tray.set_icon(Some(icon_for_status(&status)));
    let _ = state.tray.set_icon_as_template(false);
    let _ = state.tray.set_tooltip(Some(presentation.tooltip));

    let should_rebuild_menu = state
        .menu_refresh
        .lock()
        .map(|mut refresh| {
            refresh.should_rebuild(MenuContentKey::from_status(&status, can_mark_handled))
        })
        .unwrap_or(true);

    if should_rebuild_menu {
        if let Ok(menu) = build_menu(app, &status, can_mark_handled) {
            let _ = state.tray.set_menu(Some(menu));
        }
    }
}

fn force_update_tray_menu<R: Runtime>(app: &AppHandle<R>, state: &TrayAppState) {
    let notify_status = state.store.load_display_status();
    let can_mark_handled = notify_status.state == StatusState::NeedsAttention;
    let status = merge_status_parts(notify_status, &state.unread);

    let presentation = MenuBarPresentation::from_status(&status);
    let _ = state.tray.set_icon(Some(icon_for_status(&status)));
    let _ = state.tray.set_icon_as_template(false);
    let _ = state.tray.set_tooltip(Some(presentation.tooltip));
    if let Ok(menu) = build_menu(app, &status, can_mark_handled) {
        if let Ok(mut refresh) = state.menu_refresh.lock() {
            let _ = refresh.should_rebuild(MenuContentKey::from_status(&status, can_mark_handled));
        }
        let _ = state.tray.set_menu(Some(menu));
    }
}

fn merge_status_parts(
    notify_status: DisplayStatus,
    unread: &Arc<Mutex<UnreadTracker>>,
) -> DisplayStatus {
    let unread = unread
        .lock()
        .map(|tracker| tracker.snapshot())
        .unwrap_or_default();
    merge_display_status(notify_status, unread)
}

fn mark_handled_if_needed<R: Runtime>(app: &AppHandle<R>, state: &TrayAppState) {
    if state.store.load_display_status().state != StatusState::NeedsAttention {
        return;
    }

    if state.store.mark_handled().is_ok() {
        update_tray(app, state);
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    status: &DisplayStatus,
    can_mark_handled: bool,
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "status-title",
        &status.title,
        false,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "status-detail",
        &status.detail,
        false,
        None::<&str>,
    )?)?;

    if let Some(cwd) = &status.cwd {
        menu.append(&MenuItem::with_id(
            app,
            "status-cwd",
            format!("Project: {cwd}"),
            false,
            None::<&str>,
        )?)?;
    }

    if let Some(updated_at) = &status.updated_at {
        menu.append(&MenuItem::with_id(
            app,
            "status-updated-at",
            format!("Updated: {updated_at}"),
            false,
            None::<&str>,
        )?)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        MENU_OPEN_CODEX,
        "Open Codex",
        true,
        None::<&str>,
    )?)?;

    if can_mark_handled {
        menu.append(&MenuItem::with_id(
            app,
            MENU_MARK_HANDLED,
            "Mark Handled",
            true,
            None::<&str>,
        )?)?;
    }

    menu.append(&MenuItem::with_id(
        app,
        MENU_REFRESH,
        "Refresh",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        MENU_QUIT,
        "Quit",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

fn icon_for_status(status: &DisplayStatus) -> Image<'static> {
    let presentation = MenuBarPresentation::from_status(status);
    let tint = match presentation.tint {
        MenuBarTint::Idle => [255, 255, 255, 240],
        MenuBarTint::Attention => [35, 191, 111, 255],
        MenuBarTint::Warning => [255, 204, 0, 255],
    };

    match status.state {
        StatusState::Idle => circle_icon(tint),
        StatusState::NeedsAttention => message_icon(tint),
        StatusState::Error => warning_icon(tint),
    }
}

fn circle_icon(color: [u8; 4]) -> Image<'static> {
    let mut canvas = Canvas::new(32, 32);
    let line = [color[0], color[1], color[2], 235];
    canvas.stroke_smooth_circle(16.0, 16.0, 9.6, 2.35, line);
    canvas.fill_smooth_circle(16.0, 16.0, 2.55, [color[0], color[1], color[2], 185]);
    canvas.into_image()
}

fn message_icon(color: [u8; 4]) -> Image<'static> {
    let mut canvas = Canvas::new(32, 32);
    let line = [color[0], color[1], color[2], 245];
    canvas.stroke_smooth_rounded_rect(4.8, 8.9, 25.5, 24.8, 5.5, 2.25, line);
    canvas.draw_smooth_line(10.2, 24.2, 7.0, 27.6, 2.25, line);
    canvas.draw_smooth_line(7.0, 27.6, 12.9, 24.8, 2.25, line);
    canvas.fill_smooth_circle(25.2, 7.4, 4.65, color);
    canvas.into_image()
}

fn warning_icon(color: [u8; 4]) -> Image<'static> {
    let mut canvas = Canvas::new(32, 32);
    let line = [color[0], color[1], color[2], 245];
    canvas.stroke_smooth_triangle((16.0, 4.8), (27.8, 25.5), (4.2, 25.5), 2.2, line);
    canvas.draw_smooth_line(16.0, 11.7, 16.0, 18.6, 2.2, line);
    canvas.fill_smooth_circle(16.0, 22.9, 1.45, line);
    canvas.into_image()
}

struct Canvas {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    fn fill_smooth_circle(&mut self, cx: f32, cy: f32, radius: f32, color: [u8; 4]) {
        let left = (cx - radius - 1.0).floor() as i32;
        let right = (cx + radius + 1.0).ceil() as i32;
        let top = (cy - radius - 1.0).floor() as i32;
        let bottom = (cy + radius + 1.0).ceil() as i32;

        for y in top..=bottom {
            for x in left..=right {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let distance = (dx * dx + dy * dy).sqrt();
                let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
                self.blend_pixel(x, y, color, coverage);
            }
        }
    }

    fn stroke_smooth_circle(&mut self, cx: f32, cy: f32, radius: f32, width: f32, color: [u8; 4]) {
        let left = (cx - radius - 1.0).floor() as i32;
        let right = (cx + radius + 1.0).ceil() as i32;
        let top = (cy - radius - 1.0).floor() as i32;
        let bottom = (cy + radius + 1.0).ceil() as i32;
        let inner = (radius - width).max(0.0);

        for y in top..=bottom {
            for x in left..=right {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let distance = (dx * dx + dy * dy).sqrt();
                let outer_coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
                let inner_coverage = (inner + 0.5 - distance).clamp(0.0, 1.0);
                self.blend_pixel(
                    x,
                    y,
                    color,
                    (outer_coverage - inner_coverage).clamp(0.0, 1.0),
                );
            }
        }
    }

    fn stroke_smooth_rounded_rect(
        &mut self,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        radius: f32,
        width: f32,
        color: [u8; 4],
    ) {
        let cx = (left + right) / 2.0;
        let cy = (top + bottom) / 2.0;
        let half_width = (right - left) / 2.0;
        let half_height = (bottom - top) / 2.0;
        let inner_half_width = half_width - radius;
        let inner_half_height = half_height - radius;

        for y in (top - width - 1.0).floor() as i32..=(bottom + width + 1.0).ceil() as i32 {
            for x in (left - width - 1.0).floor() as i32..=(right + width + 1.0).ceil() as i32 {
                let px = x as f32 + 0.5 - cx;
                let py = y as f32 + 0.5 - cy;
                let qx = px.abs() - inner_half_width;
                let qy = py.abs() - inner_half_height;
                let outside_x = qx.max(0.0);
                let outside_y = qy.max(0.0);
                let outside = (outside_x * outside_x + outside_y * outside_y).sqrt();
                let inside = qx.max(qy).min(0.0);
                let distance = outside + inside - radius;
                let coverage = (width / 2.0 + 0.55 - distance.abs()).clamp(0.0, 1.0);
                self.blend_pixel(x, y, color, coverage);
            }
        }
    }

    fn stroke_smooth_triangle(
        &mut self,
        a: (f32, f32),
        b: (f32, f32),
        c: (f32, f32),
        width: f32,
        color: [u8; 4],
    ) {
        self.draw_smooth_line(a.0, a.1, b.0, b.1, width, color);
        self.draw_smooth_line(b.0, b.1, c.0, c.1, width, color);
        self.draw_smooth_line(c.0, c.1, a.0, a.1, width, color);
    }

    fn draw_smooth_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [u8; 4]) {
        let steps = (((x2 - x1).hypot(y2 - y1)) * 2.0).ceil() as i32;
        for step in 0..=steps.max(1) {
            let t = step as f32 / steps.max(1) as f32;
            self.fill_smooth_circle(x1 + (x2 - x1) * t, y1 + (y2 - y1) * t, width / 2.0, color);
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: [u8; 4], coverage: f32) {
        if coverage <= 0.0 || x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        let index = ((y as u32 * self.width + x as u32) * 4) as usize;
        let src_alpha = (color[3] as f32 / 255.0) * coverage.clamp(0.0, 1.0);
        let dst_alpha = self.rgba[index + 3] as f32 / 255.0;
        let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);

        if out_alpha <= 0.0 {
            return;
        }

        for channel in 0..3 {
            let src = color[channel] as f32 / 255.0;
            let dst = self.rgba[index + channel] as f32 / 255.0;
            let out = (src * src_alpha + dst * dst_alpha * (1.0 - src_alpha)) / out_alpha;
            self.rgba[index + channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        self.rgba[index + 3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    fn into_image(self) -> Image<'static> {
        Image::new_owned(self.rgba, self.width, self.height)
    }
}

fn open_codex() {
    if let Ok(command) = std::env::var("CODEX_APP_COMMAND") {
        let mut parts = command.split_whitespace();
        if let Some(program) = parts.next() {
            let _ = Command::new(program).args(parts).spawn();
            return;
        }
    }

    #[cfg(target_os = "macos")]
    let _ = Command::new("open").args(["-a", "Codex"]).spawn();

    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd")
        .args(["/C", "start", "", "Codex"])
        .spawn();
}

#[cfg(target_os = "macos")]
fn install_packaged_notify_helper(store: &StatusStore) {
    let Some(codex_home) = store.paths().status_file.parent().map(ToOwned::to_owned) else {
        return;
    };

    let Some(helper_source) = packaged_notify_helper_path() else {
        return;
    };

    if helper_source.exists() {
        let _ = install_notify_helper(codex_home, helper_source, "codex-turn-notify");
    }
}

#[cfg(not(target_os = "macos"))]
fn install_packaged_notify_helper(_store: &StatusStore) {}

#[cfg(target_os = "macos")]
fn packaged_notify_helper_path() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let contents_dir = executable.parent()?.parent()?;
    Some(contents_dir.join("Resources").join("codex-turn-notify"))
}
