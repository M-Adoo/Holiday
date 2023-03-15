use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use ribir_builtin::builtin;

include!("./src/builtin_fields_list.rs");

fn main() -> std::io::Result<()> {
  let out_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("../docs/builtin_widget/declare_builtin_fields.md");
  let mut f = match fs::File::create(dest_path) {
    Ok(f) => f,
    Err(e) => {
      // Allow error here, because `cargo publish` will always return error.
      eprintln!("{e}");
      return Ok(());
    }
  };
  f.write_all(b"# Full builtin fields list \n\n")?;

  for w in WIDGETS.iter() {
    for field in w.fields.iter() {
      f.write_all(
        format!(
          "- {} : [`{}`] \n \t - {}\n",
          field.name, field.ty, field.doc
        )
        .as_bytes(),
      )?;
    }
  }
  for w in WIDGETS.iter() {
    for m in w.methods.iter() {
      f.write_all(format!("\n - `{}`\n \t- {}\n", m.sign, m.doc).as_bytes())?;
    }
  }

  println!("cargo:rerun-if-changed=build.rs");
  Ok(())
}
