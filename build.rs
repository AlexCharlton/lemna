use std::error::Error;
use std::path::Path;

use glob::glob;
use shaderc;

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(feature = "docs_rs") {
        println!("Skipping build because we're running on docs.rs");
        return Ok(());
    }
    let compiler = shaderc::Compiler::new().unwrap();
    let path = Path::new("./src/render/wgpu/pipelines/shaders");

    for entry in glob(&format!("{}/*.[vf][er][ra][tg]", path.display())).unwrap() {
        if let Ok(file_path) = entry {
            println!(
                "cargo:rerun-if-changed={}",
                file_path.canonicalize().unwrap().display()
            );
        }
    }

    for entry in glob(&format!("{}/*.[vf][er][ra][tg]", path.display())).unwrap() {
        if let Ok(file_path) = entry {
            let shader = std::fs::read_to_string(&file_path).unwrap();
            let shader_type = if file_path.extension().unwrap() == "vert" {
                shaderc::ShaderKind::Vertex
            } else {
                shaderc::ShaderKind::Fragment
            };
            let spv = compiler
                .compile_into_spirv(
                    &shader,
                    shader_type,
                    file_path.to_str().unwrap(),
                    "main",
                    None,
                )
                .unwrap();
            let mut out_file =
                std::fs::File::create(format!("{}.spv", file_path.display())).unwrap();
            std::io::copy(&mut spv.as_binary_u8(), &mut out_file).unwrap();
        }
    }

    Ok(())
}
