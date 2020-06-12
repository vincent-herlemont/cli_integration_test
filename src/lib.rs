use assert_cmd::Command;
use fs_extra::dir::create_all;
use fs_extra::file::read_to_string;
use fs_extra::file::write_all;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use tempdir::TempDir;
use walkdir::WalkDir;
use std::os::unix::fs::PermissionsExt;
use std::fs::{set_permissions, Permissions};
use std::io;

pub struct IntegrationTestEnvironment {
    label: String,
    tmp_dir: TempDir,
    entries: HashMap<PathBuf, Option<String>>,
    cfg_command_callback: Box<dyn Fn(PathBuf,Command) -> Command>,
}

impl IntegrationTestEnvironment {
    pub fn new<L>(label: L) -> Self
        where
            L: AsRef<str>,
    {
        let label = label.as_ref().to_string();
        let tmp_dir = TempDir::new(&label).expect("fail to create tmp directory");
        Self {
            label,
            tmp_dir,
            entries: HashMap::new(),
            cfg_command_callback: Box::new(|_,c|c),
        }
    }

    pub fn set_cfg_command_callback(&mut self, callback: impl Fn(PathBuf,Command) -> Command + 'static) {
        self.cfg_command_callback = Box::new(callback);
    }

    pub fn add_file<P, C>(&mut self, path: P, content: C)
        where
            P: AsRef<Path>,
            C: AsRef<str>,
    {
        self.entries.insert(
            path.as_ref().to_path_buf(),
            Some(content.as_ref().to_string()),
        );
    }

    pub fn read_file<P>(&self, path: P) -> String
        where
            P: AsRef<Path>,
    {
        let path = self.tmp_dir.path().join(path.as_ref());
        read_to_string(&path).expect(format!("fail to read file {:?}", path).as_str())
    }

    pub fn add_dir<P>(&mut self, path: P)
        where
            P: AsRef<Path>,
    {
        self.entries.insert(path.as_ref().to_path_buf(), None);
    }

    pub fn setup(&self) {
        for (path, content) in self.entries.iter() {
            let path = self.tmp_dir.path().join(path);
            if let Some(content) = content {
                if let Some(path) = path.parent() {
                    create_all(path, false)
                        .expect(format!("fail to create directory {:?}", path).as_str())
                }
                write_all(path, content).expect("fail to create file");
            } else {
                create_all(&path, false)
                    .expect(format!("fail to create directory {:?}", path).as_str())
            }
        }
    }

    pub fn set_exec_permission<P:AsRef<Path>>(&self,file: P) -> io::Result<()> {
        let file = self.tmp_dir.path().join(file.as_ref());
        let permissions = Permissions::from_mode(0o755);
        set_permissions(file, permissions)?;
        Ok(())
    }

    pub fn tree(&self) -> Vec<PathBuf> {
        let mut tree: Vec<PathBuf> = WalkDir::new(self.tmp_dir.path())
            .into_iter()
            .filter_map(|dir_entry| {
                if let Ok(dir_entry) = dir_entry {
                    if let Ok(dir_entry) = dir_entry.path().strip_prefix(self.tmp_dir.path()) {
                        return Some(dir_entry.to_path_buf());
                    }
                }
                None
            })
            .collect();
        tree.sort();
        tree
    }

    pub fn command<C>(&self, crate_name: C) -> Command
        where
            C: AsRef<str>,
    {
        let mut command = Command::cargo_bin(crate_name).unwrap();
        command.current_dir(&self.tmp_dir.path());
        (self.cfg_command_callback)(self.path().clone().to_path_buf(),command)
    }

    pub fn path(&self) -> &Path {
        self.tmp_dir.path()
    }
}

impl Display for IntegrationTestEnvironment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for e in self.tree() {
            writeln!(f, "{}", e.to_string_lossy())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use predicates::prelude::Predicate;
    use predicates::str::contains;
    use crate::IntegrationTestEnvironment;

    #[test]
    fn integration_test_environment() {
        let mut e = IntegrationTestEnvironment::new("test");
        e.add_file("file1", "test 1");
        e.add_file("dir/file2", "test 2");
        e.add_dir("emptry_dir");
        e.setup();
        e.set_exec_permission("dir/file2").unwrap();
        let display = e.to_string();
        assert!(contains("file1").eval(display.as_str()));
        assert!(contains("dir/file2").eval(display.as_str()));
        assert!(contains("emptry_dir").eval(display.as_str()));
        assert!(contains("test 1").eval(e.read_file("file1").as_str()));
    }
}



#[macro_export]
macro_rules! println_output {
    ($v:ident) => {
        println!(
            "{}",
            String::from_utf8($v.stderr.clone()).expect("fail to read stderr")
        );
        println!("---------------------------");
        println!(
            "{}",
            String::from_utf8($v.stdout.clone()).expect("fail to read stdout")
        );
        println!("---------------------------");
        println!("{}", $v.status);
    };
}

#[macro_export]
macro_rules! println_result_output {
    ($v:ident) => {
        match $v {
            Ok(output) => {
                println_output!(output);
            }
            Err(outputError) => {
                println!("output error !!");
                println!("{}", outputError.to_string());
            }
        }
    };
}
