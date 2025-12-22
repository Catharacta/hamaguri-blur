use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, IsWindowVisible,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub fn get_active_window_info(exclude_hwnd: Option<isize>) -> Option<WindowInfo> {
    unsafe {
        let mut hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            println!("DEBUG: GetForegroundWindow returned invalid HWND");
            return None;
        }

        // 自ウィンドウを除外して背後のウィンドウを探すループ
        loop {
            if hwnd.is_invalid() {
                break;
            }

            let mut text: [u16; 512] = [0; 512];
            let len = GetWindowTextW(hwnd, &mut text);
            let title = String::from_utf16_lossy(&text[..len as usize]);

            let mut class_text: [u16; 512] = [0; 512];
            let class_len = GetClassNameW(hwnd, &mut class_text);
            let class_name = String::from_utf16_lossy(&class_text[..class_len as usize]);

            let visible = IsWindowVisible(hwnd).as_bool();
            let is_excluded = exclude_hwnd.map_or(false, |ex| hwnd.0 as isize == ex);

            // 除外すべき特殊なクラス名
            let is_systemic = class_name == "Progman"
                || class_name == "WorkerW"
                || class_name == "Shell_TrayWnd"
                || class_name == "Shell_SecondaryTrayWnd"
                || class_name == "Windows.UI.Core.CoreWindow"
                || class_name.contains("EdgeUiInputTopWndClass");

            if is_excluded {
                println!("DEBUG: Found current window (overlay) in foreground. Checking next...");
            } else if is_systemic {
                println!(
                    "DEBUG: Skipping systemic window: '{}' ({})",
                    title, class_name
                );
            } else if visible
                && !title.is_empty()
                && class_name != "ComboBox"
                && class_name != "tooltips_class32"
            {
                // 有効なウィンドウが見つかった
                println!(
                    "DEBUG: Found target window: '{}' ({}), HWND: {:?}, Visible: {}",
                    title, class_name, hwnd, visible
                );
                break;
            } else {
                println!(
                    "DEBUG: Skipping window - Title: '{}' ({}), HWND: {:?}, Visible: {}, Excluded: {}",
                    title, class_name, hwnd, visible, is_excluded
                );
            }

            // 次の（下の）ウィンドウへ
            hwnd = windows::Win32::UI::WindowsAndMessaging::GetWindow(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GW_HWNDNEXT,
            )
            .unwrap_or_default();
        }

        if hwnd.is_invalid() {
            println!("DEBUG: No suitable background window found.");
            return None;
        }

        let mut rect = RECT::default();
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );

        let mut win_rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut win_rect);

        let mut text: [u16; 512] = [0; 512];
        let len = GetWindowTextW(hwnd, &mut text);
        let title = String::from_utf16_lossy(&text[..len as usize]);

        println!("DEBUG: Boundary Comparison for '{}':", title);
        println!(
            "  DWM (Visible): L:{}, T:{}, R:{}, B:{} ({}x{})",
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            rect.right - rect.left,
            rect.bottom - rect.top
        );
        println!(
            "  Win32 (Full): L:{}, T:{}, R:{}, B:{} ({}x{})",
            win_rect.left,
            win_rect.top,
            win_rect.right,
            win_rect.bottom,
            win_rect.right - win_rect.left,
            win_rect.bottom - win_rect.top
        );
        println!(
            "  Offset (DWM - Win32): L:{}, T:{}, R:{}, B:{}",
            rect.left - win_rect.left,
            rect.top - win_rect.top,
            rect.right - win_rect.right,
            rect.bottom - win_rect.bottom
        );

        Some(WindowInfo {
            hwnd: hwnd.0 as isize,
            title,
            rect: Rect {
                left: rect.left - 1,
                top: rect.top - 1,
                right: rect.right + 1,
                bottom: rect.bottom + 1,
            },
        })
    }
}
