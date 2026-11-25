// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, error::Error, path::PathBuf, rc::Rc};

use ini::Ini;
use rfd::FileDialog;
use slint::{Image, Rgb8Pixel, SharedPixelBuffer, SharedString};

slint::include_modules!();

struct Project {
    loaded: bool,
    version: String,
    mappings: Vec<MappingEntry>,

    selected: isize,
}

enum MappingEntry {
    Folder(String, Vec<MappingEntry>),
    Map(Box<dyn Mapping>),
}

struct MappingPuyo {
    frame: SharedPixelBuffer::<Rgb8Pixel>,

    map: Vec<u8>,
    map_width: usize,
    map_height: usize,
    art: Vec<u8>,
    pal: Vec<u8>,
    format: MappingPuyoFormat,
}

enum MappingPuyoFormat {
    Byte,
    Word,
    Pal,
}

trait Mapping {
    fn render(&mut self) -> SharedPixelBuffer<Rgb8Pixel>;
    fn set_tile(&mut self, tile: usize, x: usize, y: usize);
}
impl Mapping for MappingPuyo {
    fn render(&mut self) -> SharedPixelBuffer<Rgb8Pixel> {
        let frame = self.frame.make_mut_bytes();
        let width = self.map_width * 8;
        let height = self.map_height * 8;

        for yt in 0..self.map_height {
            for xt in 0..self.map_width {
                let index = ((yt * self.map_width) + xt) * 2;
                let entry = 0x100 | (self.map[index] as u16) << 8 | self.map[index + 1] as u16;
                let tile = entry as usize * 0x20;
                for y in 0..8 {
                    for x in 0..8 {
                        let byte = self.art[tile + (y * 4) + (x / 2)];
                        let color = (byte as usize >> 4 * ((x & 1) ^ 1)) & 0xF;
                        let c = (self.pal[color * 2] as u16) << 8 | self.pal[(color * 2) + 1] as u16;

                        frame[((yt * 8 + y) * width + (xt * 8 + x)) * 3] = (((c) & 0xF) << 4) as u8;
                        frame[((yt * 8 + y) * width + (xt * 8 + x)) * 3 + 1] = (((c >> 4) & 0xF) << 4) as u8;
                        frame[((yt * 8 + y) * width + (xt * 8 + x)) * 3 + 2] = (((c >> 8) & 0xF) << 4) as u8;
                    }
                }
            }
        }
        
        self.frame.clone()
    }

    fn set_tile(&mut self, tile: usize, x: usize, y: usize) {
        if x < self.map_width && y < self.map_height {
            self.map[((y * self.map_width) + (x)) * 2 + 1] = tile as u8;
        }
    }
}

impl Project {
    fn load(ui: AppWindow, s: &Rc<RefCell<Self>>, filename: String) -> Result<(), Box<dyn Error>> {
        let mut project = s.borrow_mut();
        let ini = Ini::load_from_file(filename.clone())?;
        // TODO: Check this unwrap for potential issues.
        let base = PathBuf::from(filename).parent().unwrap().to_string_lossy().to_string() + "/";

        for (sec, prop) in ini.iter() {
            println!("Section: {:?} {:?}", sec, prop);
            if let Some(folder_name) = sec {
                /* 
                let mut folder = -1;
                for (i, m) in project.mappings.iter().enumerate() {
                    if let MappingEntry::Folder(name, map) = m && name == folder_name {
                        folder = i as isize;
                        // Add Mapping Here.
                    }
                }

                if folder == -1 {
                    project.mappings.push(MappingEntry::Folder(folder_name.to_string(), Vec::new()));
                    folder = project.mappings.len() as isize - 1;
                }
                */

                // TODO: Change base.clone() to something better.
                let map_width = prop.get("width").ok_or("Mapping entry does not contain 'width' field.")?.parse()?;
                let map_height = prop.get("height").ok_or("Mapping entry does not contain 'height' field.")?.parse()?;
                project.mappings.push(MappingEntry::Map(Box::new(MappingPuyo {
                    frame: SharedPixelBuffer::<Rgb8Pixel>::new(map_width as u32 * 8, map_height as u32 * 8),
                    map_width,
                    map_height,
                    map: std::fs::read(base.clone() + prop.get("mappings").ok_or("Mapping entry does not contain 'map' field.")?)?,
                    art: std::fs::read(base.clone() + prop.get("tiles").ok_or("Mapping entry does not contain 'tiles' field.")?)?,
                    pal: std::fs::read(base.clone() + prop.get("palette").ok_or("Mapping entry does not contain 'palette' field.")?)?,
                    format: match prop.get("format").ok_or("Mapping entry does not contain 'format' field.")? {
                        "Byte" => MappingPuyoFormat::Byte,
                        "Word" => MappingPuyoFormat::Word,
                        "Pal" => MappingPuyoFormat::Pal,
                        _ => return Err("Mapping 'format' field is set to an invalid value.".into()),
                    }
                })));
            } else {
                if let Some(version) = prop.get("version") {
                    project.version = version.to_string();
                } else {
                    return Err("Project INI contains no 'version' field.".into());
                }
            }
        }

        // DBG: remove later
        if let MappingEntry::Map(m) = &mut project.mappings[0] {
            ui.set_plane_view(Image::from_rgb8(m.render()))
        }

        Ok(())
    }
    
    fn init_callbacks(ui: &AppWindow, s: &Rc<RefCell<Self>>) {
        ui.on_open_project_dialog(move || {
            let files = FileDialog::new()
                .add_filter("project", &["ini"])
                .set_directory("/")
                .pick_file();

            if let Some(f) = files {
                SharedString::from(f.to_string_lossy().to_string())
            } else {
                SharedString::new()
            }
        });

        let u = ui.as_weak();
        let p = s.clone();
        ui.on_load_project(move |filename| {
            match Project::load(u.upgrade().unwrap(), &p, filename.to_string()) {
                Ok(_) => true,
                Err(e) => {
                    println!("Err: {e:?}");
                    false
                }
            }
        });

        let u = ui.as_weak();
        let p = s.clone();
        ui.on_place_tile(move |t, x, y| {
            let mut project = p.borrow_mut();
            if let MappingEntry::Map(m) = &mut project.mappings[0] {
                m.set_tile(t as usize, x as usize, y as usize);
                u.upgrade().unwrap().set_plane_view(Image::from_rgb8(m.render()))
            }
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let project = Rc::new(RefCell::new(Project {
        loaded: false,
        version: "".to_string(),
        mappings: Vec::new(),
        selected: -1,
    }));


    let ui = AppWindow::new()?;
    let about_slint = AbtSlint::new()?;

    //let ui_handle = ui.as_weak();

    Project::init_callbacks(&ui, &project);

    let about_slint_weak = about_slint.as_weak();
    ui.on_display_about_slint(move || {
        if let Some(win) = about_slint_weak.upgrade() {
            let _ = win.show();
        }
    });

    //ui.set_plane_view(Image::from_rgb8(pixel_buffer));

    ui.run()?;

    Ok(())
}
