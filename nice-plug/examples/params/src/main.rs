#![windows_subsystem = "windows"]
use nice_plug::prelude::*;

use plugin_example_params::ParamsPlugin;

fn main() {
    nice_export_standalone::<ParamsPlugin>();
}
