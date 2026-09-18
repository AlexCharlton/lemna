use std::error::Error;

#[cfg(not(feature = "shaders"))]
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("Skipping building shaders");
    Ok(())
}

#[cfg(feature = "shaders")]
fn main() -> Result<(), Box<dyn Error>> {
    use glob::glob;
    use std::env;
    use std::path::Path;

    let out_dir = env::var_os("OUT_DIR").ok_or("OUT_DIR not set")?;
    let out_shaders = Path::new(&out_dir).join("shaders");
    std::fs::create_dir_all(&out_shaders)?;

    let compiler = shaderc::Compiler::new().map_err(|e| e.to_string())?;
    let shaders_dir = Path::new("src/render/gpu_render/wgpu/pipelines/shaders");

    // Relative paths only — canonicalize()/UNC paths can make Cargo treat inputs as
    // missing and re-run the build script on every build.
    println!("cargo:rerun-if-changed={}", shaders_dir.display());

    for file_path in glob(&format!("{}/*.[vf][er][ra][tg]", shaders_dir.display()))?.flatten() {
        println!("cargo:rerun-if-changed={}", file_path.display());

        let shader = std::fs::read_to_string(&file_path)?;
        let shader_type = if file_path.extension().is_some_and(|ext| ext == "vert") {
            shaderc::ShaderKind::Vertex
        } else {
            shaderc::ShaderKind::Fragment
        };
        let spv = compiler.compile_into_spirv(
            &shader,
            shader_type,
            file_path.to_str().ok_or("shader path is not UTF-8")?,
            "main",
            None,
        )?;

        let file_name = file_path
            .file_name()
            .ok_or("shader path has no file name")?
            .to_string_lossy();
        let out_path = out_shaders.join(format!("{file_name}.spv"));
        let new_bytes = spv.as_binary_u8();

        // Avoid rewriting identical output (keeps OUT_DIR quieter across no-op runs).
        let should_write = match std::fs::read(&out_path) {
            Ok(existing) => existing != new_bytes,
            Err(_) => true,
        };
        if should_write {
            std::fs::write(&out_path, new_bytes)?;
        }
    }

    Ok(())
}
