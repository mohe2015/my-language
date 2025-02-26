
use gtk::{
    Application, ApplicationWindow, Button, TextView,
    gio::prelude::{ApplicationExt as _, ApplicationExtManual as _},
    prelude::{ButtonExt as _, GtkWindowExt as _, TextBufferExt, TextBufferExtManual, TextViewExt},
};
    
fn main() {
    // https://github.com/emilk/egui?tab=readme-ov-file
    let application = Application::builder()
        .application_id("com.example.FirstGtkApp")
        .build();

    application.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("My Language")
            .default_width(350)
            .default_height(70)
            .build();

        // https://docs.gtk.org/gtk4/section-text-widget.html
        let text_view = TextView::builder().build();

        let buffer = text_view.buffer();
        buffer.set_text("Hello world!");

        let tag = buffer
            .create_tag(Some("test_tag"), &[("background", &"blue".to_string())])
            .unwrap();

        let start = buffer.iter_at_offset(0);
        let end = buffer.iter_at_offset(10);
        buffer.apply_tag(&tag, &start, &end);

        window.set_child(Some(&text_view));

        window.present();
    });

    application.run();
}
