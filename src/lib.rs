use colored::{ColoredString, Colorize};
use std::{
    ffi::OsStr,
    fs::{self, DirEntry},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::Command,
};

pub struct ProjectGenArgs {
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub package: String,
    pub modid: String,
    pub entry_point: String,
    pub version_is_commit: bool,
}

impl ProjectGenArgs {
    pub fn from_user_input() -> Self {
        let version = get_complete_string(&"Minecraft version: ".green()).replace("1.16", "master");
        let name = get_complete_string(&"Name: ".green());
        let description = get_complete_string(&"Description: ".green());
        let author = get_complete_string(&"Author: ".green());
        let package = get_complete_string(&"Java package: ".green());
        let modid = get_complete_string(&"Mod ID: ".green());
        let entry_point = get_complete_string(&"Entry point name: ".green());
        let version_is_commit = version.len() == 40;

        Self {
            version,
            name,
            description,
            author,
            package,
            modid,
            entry_point,
            version_is_commit,
        }
    }
}

fn get_string(prompt: &ColoredString) -> String {
    let mut output = String::new();
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    match io::stdin().read_line(&mut output) {
        Ok(_) => output,
        Err(_) => String::new(),
    }
}

fn get_complete_string(prompt: &ColoredString) -> String {
    let out = get_string(prompt);
    if out.is_empty() {
        get_complete_string(prompt)
    } else {
        out.trim().to_string()
    }
}

fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry) -> io::Result<()>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry)?;
            }
        }
    }
    Ok(())
}

fn rename_dir_tree(old: &PathBuf, new: &PathBuf) -> io::Result<()> {
    let mut zipped = old.iter().zip(new.iter());
    let zipped_count = zipped.clone().count();
    let diff_index = zipped
        .position(|(old, new)| old != new)
        .unwrap_or(zipped_count);

    let diff_root = old
        .components()
        .map(Component::as_os_str)
        .take(diff_index)
        .collect::<PathBuf>();
    let remove_old = old
        .components()
        .map(Component::as_os_str)
        .take(diff_index + 1)
        .collect::<PathBuf>();
    let new_parent = new.parent().unwrap();
    let temp_path = diff_root.join("__TEMP_TREE_RENAME__");

    fs::rename(&old, &temp_path)?;
    fs::remove_dir_all(&remove_old)?;
    fs::create_dir_all(&new_parent)?;
    fs::rename(&temp_path, &new)?;

    Ok(())
}

pub fn make_mod(args: &ProjectGenArgs) -> io::Result<()> {
    println!(
        "{}",
        format!(
            "Cloning FabricMC/fabric-example-mod (branch {}) into {}...",
            if args.version_is_commit {
                "master"
            } else {
                &args.version
            },
            &args.modid,
        )
        .dimmed()
    );
    let root = clone_example_mod_path(args)?;

    println!("{}", "Changing files...".dimmed());
    change_gradle_properties(args, &root)?;

    let src_path = root.join("src").join("main");
    change_package(args, &src_path)?;
    change_mod_jsons(args, &src_path)?;

    println!("{}", "Done!".bold().bright_green());
    Ok(())
}

fn clone_example_mod_path(args: &ProjectGenArgs) -> io::Result<PathBuf> {
    match Command::new("git")
        .args([
            "clone",
            "-b",
            if args.version_is_commit {
                "master"
            } else {
                &args.version
            },
            "https://github.com/FabricMC/fabric-example-mod",
            &args.modid,
        ])
        .output()
    {
        Ok(o) => {
            if !o.status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    String::from_utf8(o.stderr).unwrap(),
                ));
            }

            let path = fs::canonicalize(&args.modid)?;
            fs::remove_dir_all(path.join(".git"))?;
            Ok(path)
        }
        Err(e) => Err(e),
    }
}

fn change_gradle_properties(args: &ProjectGenArgs, root: &PathBuf) -> io::Result<()> {
    let path = root.join("gradle.properties");
    let dot_index = args.package.rfind('.').unwrap_or(0);
    fs::write(
        &path,
        fs::read_to_string(&path)?
            .replace("com.example", &args.package[..dot_index])
            .replace("fabric-example-mod", &args.package[dot_index + 1..]),
    )
}

fn change_package(args: &ProjectGenArgs, src_path: &PathBuf) -> io::Result<()> {
    let resources = src_path.join("resources");
    let java_path = src_path.join("java");
    let assets = resources.join("assets");

    let mut default_package = java_path.clone();
    default_package.extend(["net", "fabricmc", "example"]);
    let mut new_package = java_path.clone();
    new_package.extend(args.package.split('.'));

    rename_dir_tree(&default_package, &new_package)?;
    fs::rename(assets.join("modid"), assets.join(&args.modid))?;

    visit_dirs(&new_package, &|entry| {
        let mut path = entry.path();
        if path.extension() != Some(OsStr::new("java")) {
            return Ok(());
        }
        if path.file_name().unwrap() == "ExampleMod.java" {
            let new_path = path
                .parent()
                .unwrap()
                .join(format!("{}.java", &args.entry_point));
            fs::rename(&path, &new_path)?;
            path = new_path;
        }

        fs::write(
            &path,
            fs::read_to_string(&path)?
                .replace("net.fabricmc.example", &args.package)
                .replace("modid", &args.modid)
                .replace("ExampleMod", &args.entry_point),
        )
    })?;

    Ok(())
}

fn change_mod_jsons(args: &ProjectGenArgs, src_path: &PathBuf) -> io::Result<()> {
    let resources = src_path.join("resources");

    let fabric_mod_json = resources.join("fabric.mod.json");
    fs::write(
        &fabric_mod_json,
        fs::read_to_string(&fabric_mod_json)?
            .replace("modid", &args.modid)
            .replace("Example Mod", &args.modid)
            .replace(
                "This is an example description! Tell everyone what your mod is about!",
                &args.description,
            )
            .replace("Me!", &args.author)
            .replace("net.fabricmc.example", &args.package)
            .replace("ExampleMod", &args.entry_point),
    )?;

    let mixins_json = resources.join(format!("{}.mixins.json", &args.modid));
    fs::rename(resources.join("modid.mixins.json"), &mixins_json)?;
    fs::write(
        &mixins_json,
        fs::read_to_string(&mixins_json)?.replace("net.fabricmc.example", &args.package),
    )?;

    Ok(())
}
