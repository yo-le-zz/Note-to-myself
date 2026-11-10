// Sous Windows, intègre l'icône dans l'exécutable. Ailleurs, ne fait rien.
fn main() {
    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("packaging/windows/note-to-myself.ico");
        resource.set("ProductName", "Note to Myself");
        resource.set("FileDescription", "Note to Myself");
        resource.set("LegalCopyright", "yolezz");
        if let Err(error) = resource.compile() {
            // Une icône manquante ne doit pas empêcher la compilation.
            println!("cargo:warning=could not embed the Windows icon: {error}");
        }
    }
}
