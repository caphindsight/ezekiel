use serde::{Serialize, Deserialize};

fn read_metadata_full(path: &str) -> serde_yaml::Value {
  let file = std::fs::File::open(path).unwrap();
  let yaml = serde_yaml::from_reader(&file).unwrap();
  yaml
}

const SEPARATOR: &'static str = "---";

fn read_metadata_preamble(path: &str) -> serde_yaml::Value {
  use std::io::BufRead;
  let file = std::fs::File::open(path).unwrap();
  let lines = std::io::BufReader::new(file).lines();
  let mut yaml_str = String::new();
  let mut parsing_preamble = false;
  for line_mb in lines {
    let line = line_mb.unwrap();
    if parsing_preamble {
      if line == SEPARATOR {
        return serde_yaml::from_str(&yaml_str).unwrap();
      } else {
        yaml_str.push_str(&line);
        yaml_str.push_str("\n");
      }
    } else {
      if line.is_empty() {
        continue;
      } else if line == SEPARATOR {
        parsing_preamble = true;
      } else {
        return serde_yaml::from_str("").unwrap();
      }
    }
  }
  serde_yaml::from_str("").unwrap()
}

// Map from relative file paths to metadata objects.
type MetadataMap = std::collections::HashMap::<String, serde_yaml::Value>;

fn is_visible(path: &std::path::Path) -> bool {
  if let Some(parent) = path.parent() {
    if !is_visible(parent) { return false; }
  }
  if path.file_name().is_none() { return true; }
  let filename = path.file_name().unwrap().to_str().unwrap();
  if filename == "www" || filename.starts_with(".") {
    return false;
  }
  true
}
fn is_public(path: &std::path::Path) -> bool {
  if let Some(parent) = path.parent() {
    if !is_public(parent) { return false; }
  }
  if path.file_name().is_none() { return true; }
  let filename = path.file_name().unwrap().to_str().unwrap();
  if filename == "www" || filename.starts_with("_") {
    return false;
  }
  true
}

fn gather_metadata(dir: &str) -> MetadataMap {
  let mut metamap = MetadataMap::new();
  for entry_mb in walkdir::WalkDir::new(dir).follow_links(true) {
    let entry = entry_mb.unwrap();
    let path = entry.path();
    let mut path_str = path.to_str().unwrap();
    if let Some(path_str_adjusted) = path_str.strip_prefix(".") {
      path_str = path_str_adjusted;
    }
    let path_meta = std::fs::metadata(path).unwrap();
    if path_meta.is_dir() { continue; }
    if !is_visible(path) { continue; }
    match path.extension().and_then(|osstr| { osstr.to_str() }) {
      Some("yml") => {
        let yaml = read_metadata_full(path.to_str().unwrap());
        metamap.insert(path_str.to_string(), yaml);
      }
      Some("md") => {
        let yaml = read_metadata_preamble(path.to_str().unwrap());
        metamap.insert(path_str.to_string(), yaml);
      }
      _ => {}
    }
  }
  metamap
}

#[derive(Debug, Serialize, Deserialize)]
struct Page {
  path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Context {
  meta: MetadataMap,
  page: Page,
}

fn copy(from: &std::path::Path) {
  let mut to = std::path::PathBuf::new();
  to.push("www");
  to.push(from);
  std::fs::create_dir_all(to.parent().unwrap()).unwrap();
  std::fs::copy(from, to).unwrap();
}

fn gen_html(from: &std::path::Path, ctx: &Context, tera: &tera::Tera) {
  let mut to = std::path::PathBuf::new();
  to.push("www");
  to.push(from);
  std::fs::create_dir_all(to.parent().unwrap()).unwrap();
  let file = std::fs::File::create(to.to_str().unwrap()).unwrap();
  let mut template_name = from.to_str().unwrap();
  if let Some(template_name_adjusted) = template_name.strip_prefix("./") {
    template_name = template_name_adjusted;
  }
  tera.render_to(template_name,
      &tera::Context::from_serialize(&ctx).unwrap(),
      file).unwrap(); 
}

fn gen_markdown(from: &std::path::Path, ctx: &Context, tera: &tera::Tera) {
  let mut to = std::path::PathBuf::new();
  to.push("www");
  let mut from_str_noext = from.to_str().unwrap();
  if let Some(from_str_adjusted) = from_str_noext.strip_suffix(".md") {
    from_str_noext = from_str_adjusted;
  }
  to.push(from_str_noext.to_string() + ".html");
  let metadata = &ctx.meta[&ctx.page.path];
  if let serde_yaml::Value::Mapping(ref mapping) = *metadata {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    let partial_file_path = to.to_str().unwrap().to_string() + ".partial";
    let mut mmark = std::process::Command::new("mmark")
      .arg("--ext-footnotes")
      .arg("--ext-mathjax")
      .arg("--ext-punctuation")
      .arg("--ext-skylighting")
      .arg("--ext-toc").arg("2-3")
      .arg("-i").arg(from.to_str().unwrap())
      .arg("-o").arg(&partial_file_path)
      .spawn().expect("mmark failed to start");
    let status = mmark.wait().expect("waiting for mmark failed");
    if status.code() != Some(0) {
      panic!("mmark failed with status {:?}", status.code());
    }

    let template_name_val = mapping.get(&serde_yaml::Value::String(
          "template".to_string())).unwrap();
    let mut template_name: String;
    if let serde_yaml::Value::String(template_name_str) = template_name_val {
      template_name = template_name_str.to_string();
    } else {
      panic!("{}: template name isn't a string", from.display());
    }
    if let Some(template_name_adjusted) = template_name.strip_prefix("/") {
      template_name = template_name_adjusted.to_string();
    }
    let file = std::fs::File::create(to.to_str().unwrap()).unwrap();
    tera.render_to(&template_name,
      &tera::Context::from_serialize(&ctx).unwrap(),
      file).unwrap();
    let generated_html = std::fs::read_to_string(to.to_str().unwrap()).unwrap();
    let partial_html = std::fs::read_to_string(&partial_file_path).unwrap();
    let full_html = generated_html.replace("<!-- markdown -->", &partial_html);
    {
      let mut file = std::fs::File::create(to.to_str().unwrap()).unwrap();
      use std::io::Write;
      write!(file, "{}", &full_html);
    }

    std::fs::remove_file(&partial_file_path);
  } else {
    panic!("{}: metadata is not a mapping", from.display());
  }
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  if args.len() != 2 {
    panic!("Usage: ezekiel [build|clean]");
  }

  if &args[1] == "build" {
    let _ = std::fs::remove_dir_all("www");
    let mut context = Context {
      meta: gather_metadata("."),
      page: Page {
        path: "".to_string(),
      },
    };
    let mut tera_templates = tera::Tera::new("**.html").unwrap();
    for entry_mb in walkdir::WalkDir::new(".").follow_links(true) {
      let entry = entry_mb.unwrap();
      let path = entry.path();
      let mut path_str = path.to_str().unwrap();
      if let Some(path_str_adjusted) = path_str.strip_prefix(".") {
        path_str = path_str_adjusted;
      }
      let path_meta = std::fs::metadata(path).unwrap();
      if path_meta.is_dir() { continue; }
      if !is_visible(path) || !is_public(path) { continue; }
      context.page = Page {
        path: path_str.to_string(),
      };
      match path.extension().and_then(|osstr| { osstr.to_str() }) {
        Some("yml") => {
          println!("SKIP\t {}", path_str);
        }
        Some("html") => {
          println!("HTML\t {}", path_str);
          gen_html(path, &context, &tera_templates);
        }
        Some("md") => {
          println!("MD\t {}", path_str);
          gen_markdown(path, &context, &tera_templates);
        }
        _ => {
          println!("COPY\t {}", path_str);
          copy(path);
        }
      }
    }
  } else if &args[1] == "clean" {
    std::fs::remove_dir_all("www").unwrap();
  } else {
    panic!("Invalid command: {}", &args[1]);
  }
}
