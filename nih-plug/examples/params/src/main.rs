use lemna_nih_plug::nih_plug::prelude::*;

use plugin_example_params::ParamsPlugin;

fn main() {
    nih_export_standalone::<ParamsPlugin>();
}
