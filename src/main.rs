mod gallery;
mod gui;
mod processor;

use clap::Parser;
use eframe::{self, egui};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file or directory to process
    #[arg(short, long)]
    input: Option<String>,

    /// Output directory to save results (default: outputs)
    #[arg(short, long)]
    output: Option<String>,

    /// Run in GUI mode
    #[arg(short, long)]
    gui: bool,
}

fn main() -> opencv::Result<()> {
    let args = Args::parse();

    if args.gui {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 300.0]),
            ..Default::default()
        };

        eframe::run_native(
            "Headshot",
            options,
            Box::new(|cc| Box::new(gui::HeadshotApp::new(cc))),
        )
        .expect("Failed to start GUI");

        Ok(())
    } else {
        // Ensure input is provided for CLI mode
        let input = args.input.expect("Input path is required in CLI mode");
        let output = args.output.unwrap_or_else(|| "outputs".to_string());

        processor::process_images(&input, &output)
    }
}
