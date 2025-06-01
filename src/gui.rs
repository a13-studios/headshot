use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use crate::processor::{self, ProcessMessage};

pub struct HeadshotApp {
    input_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    processing: bool,
    progress: f32,
    error_message: Option<String>,
    rx: Option<Receiver<ProcessMessage>>,
    tx: Option<Sender<ProcessMessage>>,
    total_images: usize,
    processed_images: usize,
    total_faces: usize,
    current_file: Option<String>,
    current_faces: Option<usize>,
    min_neighbors: i32,
    min_face_size: i32,
}

impl HeadshotApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        Self {
            input_path: None,
            output_path: None,
            processing: false,
            progress: 0.0,
            error_message: None,
            rx: Some(rx),
            tx: Some(tx),
            total_images: 0,
            processed_images: 0,
            total_faces: 0,
            current_file: None,
            current_faces: None,
            min_neighbors: 3,
            min_face_size: 500,
        }
    }

    fn select_input_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Input Folder")
            .pick_folder() 
        {
            self.input_path = Some(path);
            self.error_message = None;
            self.count_images();
        }
    }

    fn select_output_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Output Folder")
            .pick_folder() 
        {
            self.output_path = Some(path);
            self.error_message = None;
        }
    }

    fn count_images(&mut self) {
        if let Some(path) = &self.input_path {
            if let Ok(entries) = processor::collect_image_files(path) {
                self.total_images = entries.len();
            }
        }
    }

    fn process_images(&mut self) {
        if self.input_path.is_none() || self.output_path.is_none() {
            self.error_message = Some("Please select both input and output folders".to_string());
            return;
        }

        let input_path = self.input_path.as_ref().unwrap().to_str().unwrap().to_string();
        let output_path = self.output_path.as_ref().unwrap().to_str().unwrap().to_string();
        let tx = self.tx.as_ref().unwrap().clone();
        let min_neighbors = self.min_neighbors;
        let min_face_size = self.min_face_size;

        self.processing = true;
        self.progress = 0.0;
        self.processed_images = 0;
        self.total_faces = 0;
        self.error_message = None;
        self.current_file = None;
        self.current_faces = None;

        thread::spawn(move || {
            if let Err(e) = processor::process_images_with_progress(&input_path, &output_path, Some(tx.clone()), min_neighbors, min_face_size) {
                tx.send(ProcessMessage::Error(e.to_string())).unwrap_or_default();
            }
        });
    }

    fn check_messages(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    ProcessMessage::Progress(filename, face_count) => {
                        self.processed_images += 1;
                        self.total_faces += face_count;
                        self.current_file = Some(filename);
                        self.current_faces = Some(face_count);
                        if self.total_images > 0 {
                            self.progress = self.processed_images as f32 / self.total_images as f32;
                        }
                    }
                    ProcessMessage::Complete => {
                        self.processing = false;
                        self.progress = 1.0;
                        self.error_message = None;
                        self.current_file = None;
                        self.current_faces = None;
                    }
                    ProcessMessage::Error(error) => {
                        self.processing = false;
                        self.error_message = Some(error);
                        self.current_file = None;
                        self.current_faces = None;
                    }
                }
            }
        }
    }
}

impl eframe::App for HeadshotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_messages();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Headshot Image Processor");
            
            ui.horizontal(|ui| {
                if ui.button("Select Input Folder").clicked() {
                    self.select_input_folder();
                }
                if let Some(path) = &self.input_path {
                    ui.label(format!("Selected: {}", path.display()));
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Select Output Folder").clicked() {
                    self.select_output_folder();
                }
                if let Some(path) = &self.output_path {
                    ui.label(format!("Selected: {}", path.display()));
                }
            });

            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Face Detection Parameters:");
                ui.add(egui::Slider::new(&mut self.min_neighbors, 3..=25).text("Min Neighbors"));
                ui.add(egui::Slider::new(&mut self.min_face_size, 10..=1000).text("Min Face Size"));
            });
            ui.add_space(10.0);

            if self.processing {
                ui.add(egui::ProgressBar::new(self.progress)
                    .show_percentage()
                    .animate(true));
                ui.label(format!("Processing: {} / {}", self.processed_images, self.total_images));
                ui.label(format!("Total faces detected: {}", self.total_faces));
                if let Some(current_file) = &self.current_file {
                    if let Some(face_count) = self.current_faces {
                        ui.label(format!("Current file: {} ({} faces)", current_file, face_count));
                    }
                }
            } else {
                let process_button = egui::Button::new("Process Images")
                    .fill(egui::Color32::from_rgb(225, 45, 0));
                if ui.add(process_button).clicked() {
                    self.process_images();
                }
            }
        });

        // Request continuous repaint while processing
        if self.processing {
            ctx.request_repaint();
        }
    }
} 