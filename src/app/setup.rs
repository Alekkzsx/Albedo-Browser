use crate::ui::AppWindow;
use slint::ComponentHandle;

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap_or_else(|| info.location().unwrap());
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };
        eprintln!("\n[CRITICAL ERROR] App panicked at '{}', {}:{}", msg, location.file(), location.line());
    }));
}

pub fn create_window() -> Result<AppWindow, Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    Ok(ui)
}
