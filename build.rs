use std::error::Error;
use std::path::Path;

use glob::glob;

#[cfg(feature = "docs_rs")]
fn main() -> Result<(), Box<dyn Error>> {
    println!("Skipping build because we're running on docs.rs");
    return Ok(());
}

#[cfg(not(feature = "docs_rs"))]
fn main() -> Result<(), Box<dyn Error>> {
    let compiler = shaderc::Compiler::new().unwrap();
    let path = Path::new("./src/render/wgpu/pipelines/shaders");

    for file_path in glob(&format!("{}/*.[vf][er][ra][tg]", path.display()))
        .unwrap()
        .flatten()
    {
        println!(
            "cargo:rerun-if-changed={}",
            file_path.canonicalize().unwrap().display()
        );
    }

    for file_path in glob(&format!("{}/*.[vf][er][ra][tg]", path.display()))
        .unwrap()
        .flatten()
    {
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
        let mut out_file = std::fs::File::create(format!("{}.spv", file_path.display())).unwrap();
        std::io::copy(&mut spv.as_binary_u8(), &mut out_file).unwrap();
    }

    Ok(())
}
