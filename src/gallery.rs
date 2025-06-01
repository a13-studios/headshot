use crossbeam::channel::{Receiver, Sender, unbounded};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::thread;

#[derive(Clone)]
pub struct PhotoEntry {
    pub path: PathBuf,
    pub thumb_tex: Option<egui::TextureHandle>,
    pub thumb_size: egui::Vec2,
    pub last_accessed: std::time::Instant,
}

impl PhotoEntry {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            thumb_tex: None,
            thumb_size: egui::Vec2::new(128.0, 128.0),
            last_accessed: std::time::Instant::now(),
        }
    }
}

pub struct Gallery {
    photos: Vec<PhotoEntry>,
    photo_map: HashMap<PathBuf, usize>,
    thumb_receiver: Option<Receiver<(PathBuf, egui::ColorImage)>>,
    thumb_sender: Option<Sender<(PathBuf, egui::ColorImage)>>,
    texture_cache: LruCache<PathBuf, egui::TextureHandle>,
    is_loading: bool,
    selected_photo: Option<usize>,
    show_lightbox: bool,
}

impl Gallery {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            photos: Vec::new(),
            photo_map: HashMap::new(),
            thumb_receiver: Some(rx),
            thumb_sender: Some(tx),
            texture_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
            is_loading: false,
            selected_photo: None,
            show_lightbox: false,
        }
    }

    pub fn load_images_from_directory<P: AsRef<Path>>(&mut self, dir: P) {
        let dir = dir.as_ref().to_path_buf();

        // Clear existing photos
        self.photos.clear();
        self.photo_map.clear();
        self.texture_cache.clear();
        self.is_loading = true;

        // Collect image files
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(
                        ext.as_str(),
                        "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "webp"
                    ) {
                        let photo = PhotoEntry::new(path.clone());
                        self.photo_map.insert(path.clone(), self.photos.len());
                        self.photos.push(photo);
                    }
                }
            }
        }

        // Start background thumbnail generation
        if let Some(tx) = self.thumb_sender.clone() {
            let photos_paths: Vec<PathBuf> = self.photos.iter().map(|p| p.path.clone()).collect();

            thread::spawn(move || {
                for path in photos_paths {
                    if let Ok(img) = image::open(&path) {
                        // Create thumbnail (max 128px on the longest side)
                        let thumb = img.thumbnail(128, 128);
                        let rgba = thumb.to_rgba8();

                        let color_img = egui::ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_flat_samples().as_slice(),
                        );

                        if tx.send((path, color_img)).is_err() {
                            break; // Channel closed
                        }
                    }
                }
            });
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        // Process incoming thumbnails
        if let Some(rx) = &self.thumb_receiver {
            let mut any_received = false;
            while let Ok((path, color_img)) = rx.try_recv() {
                if let Some(&index) = self.photo_map.get(&path) {
                    let tex_name = format!("thumb_{}", path.to_string_lossy());
                    let texture =
                        ctx.load_texture(tex_name, color_img, egui::TextureOptions::LINEAR);

                    // Update photo entry
                    if let Some(photo) = self.photos.get_mut(index) {
                        photo.thumb_tex = Some(texture.clone());
                        photo.thumb_size =
                            egui::Vec2::new(texture.size()[0] as f32, texture.size()[1] as f32);
                    }

                    // Cache the texture
                    self.texture_cache.put(path, texture);
                    any_received = true;
                }
            }

            if any_received {
                ctx.request_repaint();
            }
        }

        // Check if all thumbnails are loaded
        if self.is_loading {
            let all_loaded = self.photos.iter().all(|p| p.thumb_tex.is_some());
            if all_loaded {
                self.is_loading = false;
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut gallery_open = true;

        egui::Window::new("Gallery")
            .resizable(true)
            .default_size([800.0, 600.0])
            .open(&mut gallery_open)
            .show(ctx, |ui| {
                if self.is_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading thumbnails...");
                    });
                }

                if self.photos.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No images found in the output directory");
                    });
                    return;
                }

                // Gallery grid
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let available_width = ui.available_width();
                    let thumb_size = 140.0;
                    let cols = ((available_width / thumb_size).floor() as usize).max(1);
                    let rows = (self.photos.len() + cols - 1) / cols;

                    TableBuilder::new(ui)
                        .columns(Column::exact(thumb_size), cols)
                        .body(|mut body| {
                            body.rows(thumb_size, rows, |mut row| {
                                let row_index = row.index();
                                for col in 0..cols {
                                    let photo_index = row_index * cols + col;
                                    if let Some(photo) = self.photos.get_mut(photo_index) {
                                        row.col(|ui| {
                                            if let Some(tex) = &photo.thumb_tex {
                                                let response = ui.add_sized(
                                                    [120.0, 120.0],
                                                    egui::ImageButton::new(tex),
                                                );

                                                if response.clicked() {
                                                    self.selected_photo = Some(photo_index);
                                                    self.show_lightbox = true;
                                                }

                                                if response.hovered() {
                                                    response.on_hover_text(
                                                        photo
                                                            .path
                                                            .file_name()
                                                            .unwrap_or_default()
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    );
                                                }

                                                photo.last_accessed = std::time::Instant::now();
                                            } else {
                                                // Placeholder while loading
                                                ui.add_sized([120.0, 120.0], egui::Spinner::new());
                                            }
                                        });
                                    }
                                }
                            });
                        });
                });

                // Show image count
                ui.separator();
                ui.label(format!("Total images: {}", self.photos.len()));
            });

        // Show lightbox if selected
        if self.show_lightbox && self.selected_photo.is_some() {
            self.show_lightbox_window(ctx);
        }

        gallery_open
    }

    fn show_lightbox_window(&mut self, ctx: &egui::Context) {
        let mut lightbox_open = true;

        egui::Window::new("Image Viewer")
            .resizable(true)
            .default_size([600.0, 600.0])
            .open(&mut lightbox_open)
            .show(ctx, |ui| {
                if let Some(index) = self.selected_photo {
                    if let Some(photo) = self.photos.get(index) {
                        ui.horizontal(|ui| {
                            if ui.button("◀ Previous").clicked() && index > 0 {
                                self.selected_photo = Some(index - 1);
                            }

                            ui.label(
                                photo
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            );

                            if ui.button("Next ▶").clicked() && index < self.photos.len() - 1 {
                                self.selected_photo = Some(index + 1);
                            }
                        });

                        ui.separator();

                        // Show the thumbnail for now (could load full-res here)
                        if let Some(tex) = &photo.thumb_tex {
                            let available_size = ui.available_size();
                            let image_size = tex.size_vec2();
                            let scale = (available_size.x / image_size.x)
                                .min(available_size.y / image_size.y)
                                .min(1.0);
                            let display_size = image_size * scale;

                            ui.centered_and_justified(|ui| {
                                ui.add_sized(display_size, egui::Image::new(tex));
                            });
                        }
                    }
                }
            });

        if !lightbox_open {
            self.show_lightbox = false;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.photos.is_empty()
    }

    pub fn photo_count(&self) -> usize {
        self.photos.len()
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }
}
