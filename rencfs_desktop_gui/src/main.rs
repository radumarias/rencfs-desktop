#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::io;
use std::sync::{Mutex, OnceLock};
use diesel::SqliteConnection;
use dotenvy::dotenv;
use eframe::egui;
use static_init::dynamic;
use tokio::runtime::Runtime;
use tracing::error;

#[dynamic]
pub(crate) static RT: Runtime = Runtime::new().expect("Cannot create tokio runtime");

pub static DB_CONN: OnceLock<Mutex<SqliteConnection>> = OnceLock::new();


fn main() -> anyhow::Result<()> {
    let path = dotenv();
    match path {
        Ok(path) => println!("Loaded env file from {:?}", path),
        Err(err) => eprintln!("Error loading env file: {:?}", err),
    }
    let conn = match rencfs_desktop_common::persistence::establish_connection() {
        Ok(db) => Mutex::new(db),
        Err(err) => {
            error!(err = %err, "Error connecting to database");
            std::panic!("Error connecting to database: {:?}", err);
        }
    };
    DB_CONN.set(conn).map_err(|_| io::Error::other("cannot set connection"))?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| {

            Ok(Box::<MyApp>::default())
        }),
    ).unwrap();

    Ok(())
}

struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}