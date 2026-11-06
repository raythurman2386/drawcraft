use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=../../assets/logo/drawcraft.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../../assets/logo/drawcraft.ico")
            .set("ProductName", "DrawCraft")
            .set("FileDescription", "DrawCraft whiteboard");
        resource.compile()?;
    }
    Ok(())
}
