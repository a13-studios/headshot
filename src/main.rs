use std::fs;
use std::path::Path;
use opencv::{
    core::{Rect, Vector, Mat, Size},
    imgcodecs,
    imgproc,
    objdetect::CascadeClassifier,
    prelude::*,
    Result,
};
use opencv::core::AlgorithmHint;
use indicatif::{ProgressBar, ProgressStyle};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file or directory to process
    #[arg(short, long)]
    input: String,

    /// Output directory to save results (default: outputs)
    #[arg(short, long, default_value = "outputs")]
    output: String,
}

fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    let input_path = Path::new(&args.input);
    let dst_dir = &args.output;

    // Create output directory if it doesn't exist
    if !Path::new(dst_dir).exists() {
        fs::create_dir(dst_dir).expect("Failed to create output directory");
    }

    // Initialize the Haar cascade classifier.
    // Make sure "haarcascade_frontalface_default.xml" is in your working directory.
    let mut face_cascade = CascadeClassifier::new("haarcascade_frontalface_default.xml")
        .expect("Failed to load cascade classifier");

    // Collect image files from input (file or directory)
    let mut entries: Vec<_> = Vec::new();

    if input_path.is_file() {
        // Validate the file extension for image types
        if let Some(ext) = input_path.extension() {
            let ext = ext.to_str().unwrap().to_lowercase();
            if ext == "png" || ext == "jpg" || ext == "jpeg" {
                entries.push(input_path.to_owned());
            } else {
                eprintln!("Input file is not a supported image format.");
                return Ok(());
            }
        }
    } else if input_path.is_dir() {
        entries = fs::read_dir(input_path)
            .expect("Input directory not found")
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_str().unwrap().to_lowercase();
                        if ext == "png" || ext == "jpg" || ext == "jpeg" {
                            return Some(path);
                        }
                    }
                }
                None
            })
            .collect();
    } else {
        eprintln!("Input path is not a file or directory.");
        return Ok(());
    }

    if entries.is_empty() {
        eprintln!("No valid image files found.");
        return Ok(());
    }

    // Create a progress bar using indicatif.
    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .expect("Invalid progress bar template")
            .progress_chars("#>-")
    );

    // Process each image in the entries vector
    for path in entries {
        let filename = path.file_name().unwrap().to_str().unwrap();
        // Use the progress bar's message functionality rather than println!
        pb.set_message(format!("Processing {}...", filename));

        // Load the image (OpenCV loads in BGR format by default)
        let mut image = imgcodecs::imread(path.to_str().unwrap(), imgcodecs::IMREAD_COLOR)?;
        if image.empty() {
            pb.println(format!("Could not load image {}", filename));
            pb.inc(1);
            continue;
        }

        // Convert the image to grayscale for detection
        let mut gray = Mat::default();
        imgproc::cvt_color(
            &image,
            &mut gray,
            imgproc::COLOR_BGR2GRAY,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;

        // Detect faces with more conservative parameters:
        // - Scale factor increased for more aggressive scaling (e.g., 1.4)
        // - Minimum neighbors increased from 3 to 5
        let mut faces: Vector<Rect> = Vector::new();
        face_cascade.detect_multi_scale(
            &gray,
            &mut faces,
            1.4,    // scale factor
            5,      // min neighbors
            0,
            Size { width: 30, height: 30 },
            Size::default(),
        )?;

        if faces.len() > 0 {
            // Only take the first detected face
            let face = faces.get(0)?;
            let top = face.y;
            let left = face.x;
            let bottom = face.y + face.height;
            let right = face.x + face.width;

            // Calculate padding (1.1 times the larger of width/height)
            let width = face.width;
            let height = face.height;
            let padding = ((width.max(height)) as f64 * 1.1).round() as i32;

            // Determine the new bounding box with padding, ensuring bounds are within the image dimensions
            let padded_top = (top - padding).max(0);
            let padded_left = (left - padding).max(0);
            let padded_bottom = (bottom + padding).min(image.rows());
            let padded_right = (right + padding).min(image.cols());

            // Define the region of interest (ROI)
            let roi_width = padded_right - padded_left;
            let roi_height = padded_bottom - padded_top;
            let rect = Rect::new(padded_left, padded_top, roi_width, roi_height);

            // Crop the face clip
            let face_clip = Mat::roi(&image, rect)?;
            let face_filename = format!("{}/clip_{}", dst_dir, filename);

            // Save the cropped face image to the output directory
            imgcodecs::imwrite(&face_filename, &face_clip, &Vector::<i32>::new())?;
        }
        pb.inc(1);
    }

    pb.finish_with_message("Processing complete");
    Ok(())
}