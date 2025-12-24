mod window_manager;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SW_SHOWMAXIMIZED, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

#[tauri::command]
fn js_log(message: String) {
    println!("JS LOG: {}", message);
}

/// アクティブウィンドウの HWND を取得（ブラーウィンドウ自身を除外）
fn get_active_window_hwnd(exclude_hwnd: Option<isize>) -> Option<HWND> {
    let info = window_manager::get_active_window_info(exclude_hwnd);
    info.map(|i| HWND(i.hwnd as *mut _))
}

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

/// ブラーウィンドウをアクティブウィンドウの直下に配置
fn set_blur_below_active(blur_hwnd: HWND, active_hwnd: HWND) {
    unsafe {
        // アクティブウィンドウの直下にブラーウィンドウを配置
        let _ = SetWindowPos(
            blur_hwnd,
            Some(active_hwnd),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

fn create_blur_window(app: &AppHandle) -> tauri::Result<()> {
    let window =
        WebviewWindowBuilder::new(app, "blur_overlay", WebviewUrl::App("blur.html".into()))
            .title("hamaguri-blur")
            .decorations(false)
            .transparent(true)
            .visible(false)
            .build()?;

    let _ = window.set_ignore_cursor_events(true);

    if let Ok(hwnd) = window.hwnd() {
        set_click_through(HWND(hwnd.0));
    }

    // window-vibrancy で Acrylic ブラー効果を適用
    // RGBA: (R, G, B, A) - A は透明度（0=完全透明, 255=不透明）
    match window_vibrancy::apply_acrylic(&window, Some((18, 18, 18, 200))) {
        Ok(_) => println!("Acrylic blur effect applied successfully"),
        Err(e) => {
            println!("Failed to apply acrylic: {:?}", e);
            // フォールバック: 通常のブラーを試す
            if let Err(e2) = window_vibrancy::apply_blur(&window, Some((18, 18, 18, 200))) {
                println!("Failed to apply blur fallback: {:?}", e2);
            } else {
                println!("Blur fallback applied successfully");
            }
        }
    }

    Ok(())
}

fn start_zorder_loop(app_handle: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            if let Some(blur_window) = app_handle.get_webview_window("blur_overlay") {
                if !blur_window.is_visible().unwrap_or(false) {
                    continue;
                }

                let blur_hwnd = match blur_window.hwnd() {
                    Ok(h) => HWND(h.0),
                    Err(_) => continue,
                };

                // アクティブウィンドウを取得（ブラーウィンドウ自身を除外）
                if let Some(active_hwnd) = get_active_window_hwnd(Some(blur_hwnd.0 as isize)) {
                    set_blur_below_active(blur_hwnd, active_hwnd);
                }
            }
        }
    });
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
            create_blur_window(app.handle())?;
            start_zorder_loop(app.handle().clone());

            let alt_b_shortcut = "Alt+B".parse::<Shortcut>().unwrap();
            app.global_shortcut()
                .on_shortcut(alt_b_shortcut, |app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(blur_window) = app.get_webview_window("blur_overlay") {
                            let is_visible = blur_window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = blur_window.hide();
                                println!("Blur window hidden");
                            } else {
                                // フルスクリーン表示（Windows API で直接最大化）
                                if let Ok(hwnd) = blur_window.hwnd() {
                                    let hwnd = HWND(hwnd.0);
                                    let _ = blur_window.set_ignore_cursor_events(true);
                                    set_click_through(hwnd);
                                    unsafe {
                                        // SW_SHOWMAXIMIZED で最大化表示
                                        let _ = ShowWindow(hwnd, SW_SHOWMAXIMIZED);
                                    }
                                }
                                println!("Blur window shown");
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
        .invoke_handler(tauri::generate_handler![open_settings, js_log])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
