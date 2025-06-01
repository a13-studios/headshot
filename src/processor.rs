use opencv::core::AlgorithmHint;
use opencv::{
    Result,
    core::{Mat, Rect, Size, Vector},
    imgcodecs, imgproc,
    objdetect::CascadeClassifier,
    prelude::*,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

pub enum ProcessMessage {
    Progress(String, usize), // filename, face count for this image
    Complete,
    Error(String),
}

pub fn process_images(input: &str, output: &str) -> Result<()> {
    process_images_with_progress(input, output, None, 8, 100)
}

pub fn process_images_with_progress(
    input: &str,
    output: &str,
    progress_sender: Option<Sender<ProcessMessage>>,
    min_neighbors: i32,
    min_face_size: i32,
) -> Result<()> {
    let input_path = Path::new(input);
    let dst_dir = output;

    // Create output directory if it doesn't exist
    if !Path::new(dst_dir).exists() {
        fs::create_dir(dst_dir).expect("Failed to create output directory");
    }

    // Initialize the Haar cascade classifier
    let mut face_cascade = CascadeClassifier::new("haarcascade_frontalface_default.xml")
        .expect("Failed to load cascade classifier");

    // Collect image files
    let entries = collect_image_files(input_path)?;
    if entries.is_empty() {
        let error = "No valid image files found.";
        if let Some(sender) = &progress_sender {
            sender
                .send(ProcessMessage::Error(error.to_string()))
                .unwrap_or_default();
        } else {
            eprintln!("{}", error);
        }
        return Ok(());
    }

    // Process each image
    for path in entries {
        if let Err(e) = process_single_image(
            &path,
            dst_dir,
            &mut face_cascade,
            &progress_sender,
            min_neighbors,
            min_face_size,
        ) {
            let error_msg = format!("Error processing {}: {}", path.display(), e);
            if let Some(sender) = &progress_sender {
                sender
                    .send(ProcessMessage::Error(error_msg))
                    .unwrap_or_default();
            } else {
                eprintln!("{}", error_msg);
            }
            return Err(e);
        }
    }

    if let Some(sender) = progress_sender {
        sender.send(ProcessMessage::Complete).unwrap_or_default();
    }

    Ok(())
}

pub fn collect_image_files(input_path: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();

    if input_path.is_file() {
        if is_valid_image(input_path) {
            entries.push(input_path.to_owned());
        }
    } else if input_path.is_dir() {
        entries = fs::read_dir(input_path)
            .expect("Input directory not found")
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if is_valid_image(&path) {
                        return Some(path);
                    }
                }
                None
            })
            .collect();
    }

    Ok(entries)
}

fn is_valid_image(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap().to_lowercase();
        return ext == "png" || ext == "jpg" || ext == "jpeg";
    }
    false
}

fn process_single_image(
    path: &Path,
    dst_dir: &str,
    face_cascade: &mut CascadeClassifier,
    progress_sender: &Option<Sender<ProcessMessage>>,
    min_neighbors: i32,
    min_face_size: i32,
) -> Result<()> {
    let filename = path.file_name().unwrap().to_str().unwrap();

    // Split filename and extension
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let ext = path.extension().unwrap().to_str().unwrap();

    // Load and process image
    let image = imgcodecs::imread(path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)?;
    if image.empty() {
        return Ok(());
    }

    // Convert to grayscale
    let mut gray = Mat::default();
    imgproc::cvt_color(
        &image,
        &mut gray,
        imgproc::COLOR_BGR2GRAY,
        0,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    // Detect faces
    let mut faces: Vector<Rect> = Vector::new();
    face_cascade.detect_multi_scale(
        &gray,
        &mut faces,
        1.4,
        min_neighbors,
        0,
        Size {
            width: min_face_size,
            height: min_face_size,
        },
        Size::default(),
    )?;

    let face_count = faces.len();

    if let Some(sender) = progress_sender {
        sender
            .send(ProcessMessage::Progress(filename.to_string(), face_count))
            .unwrap_or_default();
    }

    // Process all detected faces
    for face_idx in 0..face_count {
        let face = faces.get(face_idx)?;
        let rect = calculate_padded_rect(&face, &image);

        // Crop and save the face
        let face_clip = Mat::roi(&image, rect)?;
        let face_filename = format!("{}/{}_face_{}.{}", dst_dir, stem, face_idx + 1, ext);
        imgcodecs::imwrite(&face_filename, &face_clip, &Vector::<i32>::new())?;
    }

    Ok(())
}

fn calculate_padded_rect(face: &Rect, image: &Mat) -> Rect {
    let padding = ((face.width.max(face.height)) as f64 * 1.1).round() as i32;

    let padded_top = (face.y - padding).max(0);
    let padded_left = (face.x - padding).max(0);
    let padded_bottom = (face.y + face.height + padding).min(image.rows());
    let padded_right = (face.x + face.width + padding).min(image.cols());

    let roi_width = padded_right - padded_left;
    let roi_height = padded_bottom - padded_top;

    Rect::new(padded_left, padded_top, roi_width, roi_height)
}
