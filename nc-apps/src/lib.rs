#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use nc_app::App;
use nc_app_browser::BrowserApp;
use nc_app_files::FilesApp;

pub fn builtin() -> Vec<Box<dyn App>> {
    let mut apps: Vec<Box<dyn App>> = Vec::new();
    apps.push(Box::new(FilesApp::new()));
    apps.push(Box::new(BrowserApp::new()));
    apps
}
