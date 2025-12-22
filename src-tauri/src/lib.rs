mod window_manager;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use windows::Win32::Foundation::HWND;

#[tauri::command]
fn js_log(message: String) {
    println!("JS LOG: {}", message);
}

#[tauri::command]
fn get_active_window(window: tauri::Window) -> Option<window_manager::WindowInfo> {
    let hwnd_exclude = window.hwnd().ok().map(|h| h.0 as isize);
    let mut info = window_manager::get_active_window_info(hwnd_exclude);

    if let Some(ref mut i) = info {
        let target_monitor = window
            .monitor_from_point(i.rect.left as f64, i.rect.top as f64)
            .ok()
            .flatten();

        if let Some(ref tm) = target_monitor {
            let tm_pos = tm.position();
            let tm_size = tm.size();

            if let (Ok(curr_pos), Ok(curr_size)) = (window.outer_position(), window.inner_size()) {
                if curr_pos.x != tm_pos.x
                    || curr_pos.y != tm_pos.y
                    || curr_size.width != tm_size.width
                    || curr_size.height != tm_size.height
                {
                    let _ = window.set_position(tauri::Position::Physical(*tm_pos));
                    let _ = window.set_size(tauri::Size::Physical(*tm_size));

                    // 移動・リサイズ後、確実にクリック透過を再適用
                    let _ = window.set_ignore_cursor_events(true);
                    if let Ok(hwnd) = window.hwnd() {
                        set_click_through(HWND(hwnd.0));
                    }
                    println!(
                        "Rust: Resynced overlay to monitor: {:?} at {:?}",
                        tm.name(),
                        tm_pos
                    );
                }
            }
        }

        // オーバーレイの現在の物理位置を取得
        // ターゲットモニタがある場合は、その位置（期待される位置）を基準にする
        // これにより、オーバーレイ自体の配置が数ピクセルずれていても「穴」の位置は正確になる
        let (pos_x, pos_y) = if let Some(ref tm) = target_monitor {
            let p = tm.position();
            (p.x, p.y)
        } else if let Ok(pos) = window.outer_position() {
            (pos.x, pos.y)
        } else {
            (0, 0)
        };

        // 物理ピクセル単位で減算して相対座標にする
        i.rect.left -= pos_x;
        i.rect.right -= pos_x;
        i.rect.top -= pos_y;
        i.rect.bottom -= pos_y;

        // Windows の不可視境界線や DPI スケーリングによるズレを補正するためのオフセット（物理ピクセル）
        // ユーザー報告の「右ずれ」を解消するため、左方向（マイナス）に調整
        const OFFSET_X: i32 = -7; // 左に7pxずらす
        const OFFSET_Y: i32 = 0;

        i.rect.left += OFFSET_X;
        i.rect.right += OFFSET_X;
        i.rect.top += OFFSET_Y;
        i.rect.bottom += OFFSET_Y;

        let sf = window.scale_factor().unwrap_or(1.0);
        println!(
            "Rust: Final Local Rect (Rel to {},{}, Scale:{}): L:{}, T:{}, R:{}, B:{}",
            pos_x, pos_y, sf, i.rect.left, i.rect.top, i.rect.right, i.rect.bottom
        );
    }

    info
}

use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, ShowWindow, GWL_EXSTYLE, SW_SHOWNOACTIVATE, WS_EX_LAYERED,
    WS_EX_TRANSPARENT,
};

fn set_click_through(hwnd: HWND) {
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            ex_style | (WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0) as i32,
        );
    }
}

fn create_overlay_window(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html".into()))
        .title("hamaguri-blur-overlay")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible(false)
        .build()?;

    let _ = window.set_ignore_cursor_events(true);

    if let Ok(hwnd) = window.hwnd() {
        set_click_through(HWND(hwnd.0));
    }

    Ok(())
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            create_overlay_window(app.handle())?;

            let ctrl_b_shortcut = "Alt+B".parse::<Shortcut>().unwrap();
            app.global_shortcut()
                .on_shortcut(ctrl_b_shortcut, |app, shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                        && shortcut.id() == "alt+b".parse::<Shortcut>().unwrap().id()
                    {
                        if let Some(overlay) = app.get_webview_window("overlay") {
                            let is_visible = overlay.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = overlay.hide();
                                println!("Rust: Overlay hidden by shortcut");
                            } else if let Ok(hwnd) = overlay.hwnd() {
                                // 表示するタイミングでクリック透過を再度確実にする
                                let _ = overlay.set_ignore_cursor_events(true);
                                set_click_through(HWND(hwnd.0));

                                unsafe {
                                    let _ = ShowWindow(HWND(hwnd.0), SW_SHOWNOACTIVATE);
                                }
                                println!("Rust: Overlay shown by shortcut");
                            }
                        }
                    }
                })?;

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Settings", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_settings,
            get_active_window,
            js_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
