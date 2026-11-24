// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, error::Error, rc::Rc};

use slint::{Image, Rgb8Pixel, SharedPixelBuffer};

slint::include_modules!();

struct Project {
    loaded: bool,
    map: Vec<u8>,
    map_width: usize,
    map_height: usize,
    art: Vec<u8>,
    pal: Vec<u8>,
}

impl Project {
    fn load(s: &Rc<RefCell<Self>>) -> Result<(), Box<dyn Error>> {
        let mut p = s.borrow_mut();
        p.loaded = true;
        p.map = std::fs::read("test_data/map.bin")?;
        p.map_width = 3;
        p.map_height = 3;
        p.art = std::fs::read("test_data/art.bin")?;
        p.pal = std::fs::read("test_data/pal.bin")?;

        Ok(())
    }
    
    fn init_callbacks(ui: &AppWindow, s: &Rc<RefCell<Self>>) {
        let p = s.clone();
        ui.on_load_project(move || {
            match Project::load(&p) {
                Ok(_) => true,
                Err(_) => false,
            }
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let project = Rc::new(RefCell::new(Project {
        loaded: false,
        map: Vec::new(),
        map_width: 0,
        map_height: 0,
        art: Vec::new(),
        pal: Vec::new(),
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

    // Plane view test
    let mut pixel_buffer = SharedPixelBuffer::<Rgb8Pixel>::new(320, 200);

    for y in 0..200 {
        for x in 0..320 {
            let c = x ^ y;
            pixel_buffer.make_mut_bytes()[(y*320+x)*3] = c as u8;
        }
    }

    ui.set_plane_view(Image::from_rgb8(pixel_buffer));

    ui.run()?;

    Ok(())
}
