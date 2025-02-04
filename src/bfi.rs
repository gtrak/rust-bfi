use std::{ops::Add, time::{Duration, SystemTime}};

use windows::{
    core::PCSTR,
    Wdk::Graphics::Direct3D::{
        D3DKMTOpenAdapterFromHdc, D3DKMTWaitForVerticalBlankEvent,
        D3DKMT_GETSCANLINE, D3DKMT_OPENADAPTERFROMHDC, D3DKMT_WAITFORVERTICALBLANKEVENT,
    },
    Win32::{
        Foundation::{GetLastError, COLORREF, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM},
        Graphics::Gdi::{CreateSolidBrush, GetDC, RedrawWindow, UpdateWindow, RDW_INVALIDATE},
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL},
        },
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DestroyWindow, DispatchMessageA, LoadCursorW,
            LoadIconW, PeekMessageA, PostQuitMessage, RegisterClassExA, SetLayeredWindowAttributes,
            ShowWindow, TranslateMessage, IDC_ARROW, IDI_APPLICATION, LWA_ALPHA, MSG, PM_REMOVE,
            SW_SHOWDEFAULT, SYSTEM_METRICS_INDEX, WM_CLOSE, WM_DESTROY, WNDCLASSEXA,
            WNDCLASS_STYLES, WS_EX_LAYERED, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

const CLASS_NAME: PCSTR = PCSTR::from_raw("desktopBFIwindowClass".as_bytes().as_ptr());

static mut QUIT_PROGRAM: bool = false;

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => {
            QUIT_PROGRAM = true;
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            QUIT_PROGRAM = true;
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcA(hwnd, msg, wparam, lparam),
    }
}

fn get_system_metrics(index: i32) -> i32 {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SYSTEM_METRICS_INDEX(index))
    }
}

pub(crate) fn main() {
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
        let hinstance = GetModuleHandleW(None).unwrap();

        let mut wcex = WNDCLASSEXA::default();
        wcex.cbSize = std::mem::size_of::<WNDCLASSEXA>() as u32;
        wcex.style = WNDCLASS_STYLES(0);
        wcex.lpfnWndProc = Some(wnd_proc);
        wcex.cbClsExtra = 0;
        wcex.cbWndExtra = 0;
        wcex.hInstance = hinstance.into();
        wcex.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap();
        wcex.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
        wcex.hbrBackground = CreateSolidBrush(COLORREF(0));

        wcex.lpszMenuName = PCSTR::null();
        wcex.lpszClassName = CLASS_NAME;

        if RegisterClassExA(&wcex) == 0 {
            println!("Window Registration Failed!");
            return;
        }

        let hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST,
            CLASS_NAME,
            PCSTR::from_raw("smt_off.exe".as_bytes().as_ptr()),
            WS_POPUP,
            0,
            0,
            get_system_metrics(0), // SM_CXSCREEN
            get_system_metrics(1), // SM_CYSCREEN
            None,
            None,
            hinstance,
            None,
        );

        if hwnd.is_err() {
            println!("Window Creation Failed!");
            return;
        }

        let hwnd = hwnd.unwrap();

        // A bit sus, because it seems to be passed in by the OS
        let _ = ShowWindow(hwnd, SW_SHOWDEFAULT);
        let _ = UpdateWindow(hwnd);

        let mut oa = D3DKMT_OPENADAPTERFROMHDC::default();
        oa.hDc = GetDC(hwnd);
        let result = D3DKMTOpenAdapterFromHdc(&mut oa);
        if result.is_err() {
            println!("Failed to open adapter from HDC!");
            return;
        }

        let mut we = D3DKMT_WAITFORVERTICALBLANKEVENT::default();
        we.hAdapter = oa.hAdapter;
        // not sure if necessary
        we.hDevice = 0;
        we.VidPnSourceId = oa.VidPnSourceId;

        let mut gsl = D3DKMT_GETSCANLINE::default();
        gsl.hAdapter = oa.hAdapter;
        gsl.VidPnSourceId = oa.VidPnSourceId;

        // flip every 1/120 of a second
        const INTERVAL: Duration = Duration::from_nanos(((1000f64/239.1f64)*1_000_000.0) as u64); // ~60hz


        let mut last_flip = SystemTime::now();
        let mut transparent = false;


        while !QUIT_PROGRAM {
            let _ = D3DKMTWaitForVerticalBlankEvent(&we);

            let flip = {
              if transparent {
                SystemTime::now().duration_since(last_flip).unwrap() >= INTERVAL
              } else {
                SystemTime::now().duration_since(last_flip).unwrap() >= INTERVAL.add(INTERVAL).add(INTERVAL)
              }
            };
            if flip {
                while last_flip < SystemTime::now() - INTERVAL {
                    last_flip += INTERVAL;
                }

                transparent = !transparent;

                let _ = SetLayeredWindowAttributes(
                    hwnd,
                    COLORREF(0),
                    if transparent { 0 } else { 255 },
                    LWA_ALPHA,
                );
                let err = GetLastError();
                if err != WIN32_ERROR(0) {
                    print!("{:?}", err);
                }
                let _ = RedrawWindow(hwnd, None, None, RDW_INVALIDATE);
            }

            let mut msg = MSG::default();

            while PeekMessageA(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
        }
    }
}
