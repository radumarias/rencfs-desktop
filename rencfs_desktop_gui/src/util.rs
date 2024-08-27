use egui_notify::Toast;
use std::time::Duration;

pub(crate) fn customize_toast_duration(t: &mut Toast, seconds: u64) {
    let duration = Some(Duration::from_secs(seconds));
    t.set_closable(false)
        .set_duration(duration)
        .set_show_progress_bar(false);
}

pub(crate) fn customize_toast(t: &mut Toast) {
    customize_toast_duration(t, 5);
}
