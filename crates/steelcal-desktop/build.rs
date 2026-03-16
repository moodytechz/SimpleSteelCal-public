fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-light".to_string());
    slint_build::compile_with_config("ui/app.slint", config).expect("failed to compile Slint UI");

    // Embed Windows PE metadata (icon, version, company info) into the executable.
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set("ProductName", "Simple Steel Calculator");
        res.set("FileDescription", "Simple Steel Calculator");
        res.set("CompanyName", "Harbor Pipe & Steel Inc.");
        res.set(
            "LegalCopyright",
            "Copyright \u{00A9} Harbor Pipe & Steel Inc.",
        );
        res.set("OriginalFilename", "SimpleSteelCalculator.exe");
        res.compile().expect("failed to compile Windows resources");
    }
}
