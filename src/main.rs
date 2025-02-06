use eframe::egui;
use image::{GenericImageView, RgbaImage};
use std::{fs::File, io::{self, Write, Read}};
use flate2::{write::ZlibEncoder, Compression};
use std::path::{Path, PathBuf};
use rfd::FileDialog;

#[derive(Default)]
struct ShivanoshApp {
    input_paths: Vec<PathBuf>,
    shivanosh_paths: Vec<PathBuf>,
    status: String,
    image_to_view: Option<RgbaImage>,
    texture_handle: Option<eframe::egui::TextureHandle>,
}

impl ShivanoshApp {
    fn convert_to_shivanosh(&mut self) {
        if self.input_paths.is_empty() {
            self.status = "No images selected!".to_string();
            return;
        }

        for input_path in &self.input_paths {
            let output_path = self.get_output_path(input_path);
            match Self::convert_image_to_shivanosh(input_path, &output_path) {
                Ok(_) => {
                    self.status = format!("Successfully converted: {}", input_path.display());
                }
                Err(e) => {
                    self.status = format!("Error converting {}: {e}", input_path.display());
                    break;
                }
            }
        }
    }

    fn get_output_path(&self, input_path: &Path) -> PathBuf {
        let mut output_path = input_path.to_path_buf();
        output_path.set_extension("shivanosh");
        output_path
    }

    fn convert_image_to_shivanosh(input_path: &Path, output_path: &Path) -> io::Result<()> {
        let img = image::open(input_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let (width, height) = img.dimensions();
        let mut pixel_data = Vec::new();

        for (_, _, pixel) in img.pixels() {
            pixel_data.extend_from_slice(&pixel.0); // Include alpha channel
        }

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&pixel_data)?;
        let compressed_data = encoder.finish()?;

        let mut file = File::create(output_path)?;
        file.write_all(b"MYIF")?;
        file.write_all(&width.to_le_bytes())?;
        file.write_all(&height.to_le_bytes())?;
        file.write_all(&compressed_data)?;
        Ok(())
    }

    fn open_file_dialog(&mut self) {
        if let Some(paths) = FileDialog::new().pick_files() {
            self.input_paths = paths;
        }
    }

    fn open_shivanosh_dialog(&mut self) {
        if let Some(paths) = FileDialog::new().add_filter("Shivanosh", &["shivanosh"]).pick_files() {
            self.shivanosh_paths = paths;
            self.view_shivanosh_images();
        }
    }

    fn view_shivanosh_images(&mut self) {
        if let Some(path) = self.shivanosh_paths.first() {
            match Self::decompress_shivanosh(path) {
                Ok(img) => {
                    self.image_to_view = Some(img);
                    self.texture_handle = None; // Reset texture handle
                }
                Err(e) => self.status = format!("Error viewing {}: {e}", path.display()),
            }
        }
    }

    fn decompress_shivanosh(path: &Path) -> io::Result<RgbaImage> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != b"MYIF" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid .shivanosh file"));
        }

        let mut width_bytes = [0u8; 4];
        file.read_exact(&mut width_bytes)?;
        let width = u32::from_le_bytes(width_bytes);

        let mut height_bytes = [0u8; 4];
        file.read_exact(&mut height_bytes)?;
        let height = u32::from_le_bytes(height_bytes);

        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)?;

        let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;

        let mut img = RgbaImage::new(width, height);
        let pixel_count = (width * height * 4) as usize;
        for i in 0..pixel_count / 4 {
            let r = decompressed_data[i * 4];
            let g = decompressed_data[i * 4 + 1];
            let b = decompressed_data[i * 4 + 2];
            let a = decompressed_data[i * 4 + 3];
            img.put_pixel(
                (i as u32 % width) as u32,
                (i as u32 / width) as u32,
                image::Rgba([r, g, b, a]),
            );
        }

        Ok(img)
    }
}

impl eframe::App for ShivanoshApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Selected Files:");
            if self.input_paths.is_empty() {
                ui.label("No files selected.");
            } else {
                for path in &self.input_paths {
                    ui.label(path.display().to_string());
                }
            }

            if ui.button("Select Images").clicked() {
                self.open_file_dialog();
            }

            if ui.button("Convert to .shivanosh").clicked() {
                self.convert_to_shivanosh();
            }

            ui.separator();
            ui.label(format!("Status: {}", self.status));

            if let Some(image) = &self.image_to_view {
                if self.texture_handle.is_none() {
                    let size = [image.width() as usize, image.height() as usize];
                    let pixels: Vec<eframe::egui::Color32> = image.pixels()
                        .map(|pixel| {
                            let [r, g, b, a] = pixel.0;
                            eframe::egui::Color32::from_rgba_premultiplied(r, g, b, a)
                        })
                        .collect();

                    let image_data = eframe::egui::ColorImage { size, pixels };
                    let options = eframe::egui::TextureOptions::LINEAR;
                    self.texture_handle = Some(ctx.load_texture("shivanosh_image", image_data, options));
                }

                if let Some(texture) = &self.texture_handle {
                    let available_size = ui.available_size();
                    let texture_size = texture.size_vec2();
                    let scale = (available_size.x / texture_size.x).min(available_size.y / texture_size.y);
                    ui.add(egui::Image::new(texture));  // Removed scaling
                }
            }

            if ui.button("View .shivanosh Images").clicked() {
                self.open_shivanosh_dialog();
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Shivanosh Converter",
        options,
        Box::new(|_cc| Box::new(ShivanoshApp::default())),
    )
}
