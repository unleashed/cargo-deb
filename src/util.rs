use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Get the filename from a path. Intended to be replaced when testing.
/// Note: Due to the way the Path type works the final component is returned
/// even if it looks like a directory, e.g. "/some/dir/" will return "dir"...
pub(crate) fn fname_from_path(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into()
}


cfg_if! {
    if #[cfg(not(test))] {
        pub(crate) fn is_path_file(path: &PathBuf) -> bool {
            path.is_file()
        }

        pub(crate) fn read_file_to_string(path: &Path) -> std::io::Result<String> {
            std::fs::read_to_string(path)
        }
    }
}
/// Create a HashMap from one or more key => value pairs in a single statement.
/// 
/// # Usage
/// 
/// Any types supported by HashMap for keys and values are supported:
/// ```
/// let mut one = std::collections::HashMap::new();
/// one.insert(1, 'a');
/// assert_eq!(one, map!{ 1 => 'a' });
///
/// let mut two = std::collections::HashMap::new();
/// two.insert("a", 1);
/// two.insert("b", 2);
/// assert_eq!(two, map!{ "a" => 1, "b" => 2 });
/// ```
/// 
/// Empty maps are not supported, attempting to create one will fail to compile:
/// ```compile_fail
/// let empty = std::collections::HashMap::new();
/// assert_eq!(empty, map!{ });
/// ```
/// 
/// # Provenance
/// 
/// From: https://stackoverflow.com/a/27582993
macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

/// A trait for returning a String containing items separated by the given
/// separator.
pub(crate) trait MyJoin {
    fn join(&self, sep: &str) -> String;
}

/// Returns a String containing the hash set items joined together by the given
/// separator.
/// 
/// # Usage
/// 
/// ```text
/// let two: BTreeSet<String> = vec!["a", "b"].into_iter().map(|s| s.to_owned()).collect();
/// assert_eq!("ab", two.join(""));
/// assert_eq!("a,b", two.join(","));
/// ```
impl MyJoin for BTreeSet<String> {
    fn join(&self, sep: &str) -> String {
        self.iter().map(|item| item.as_str()).collect::<Vec<&str>>().join(sep)
    }
}

cfg_if! {
    if #[cfg(test)] {
        use std::collections::HashMap;

        // ---------------------------------------------------------------------
        // Begin: test virtual filesystem
        // ---------------------------------------------------------------------
        // The pkgfile() function accesses the filesystem directly via its use
        // the Path(Buf)::is_file() method which checks for the existence of a
        // file in the real filesystem.
        //
        // To test this without having to create real files and directories we
        // extend the PathBuf type via a trait with a mock_is_file() method
        // which, in test builds, is used by pkgfile() instead of the real
        // PathBuf::is_file() method.
        //
        // The mock_is_file() method looks up a path in a vector which
        // represents a set of paths in a virtual filesystem. However, accessing
        // global state in a multithreaded test run is unsafe, plus we want each
        // test to define its own virtual filesystem to test against, not a
        // single global virtual filesystem shared by all tests.
        //
        // This test specific virtual filesystem is implemented as a map,
        // protected by a thread local such that each test (thread) gets its own
        // instance. To be able to mutate the map it is wrapped inside a Mutex.
        // To make this setup easier to work with we define a few  helper
        // functions:
        //
        //   - add_test_fs_paths() - adds paths to the current tests virtual fs
        //   - set_test_fs_path_content() - set the file content (initially "")
        //   - with_test_fs() - passes the current tests virtual fs vector to
        //                      a user defined callback function.
        use std::sync::Mutex;

        thread_local!(
            static MOCK_FS: Mutex<HashMap<&'static str, String>> = Mutex::new(HashMap::new())
        );

        pub(crate) fn add_test_fs_paths(paths: &Vec<&'static str>) {
            MOCK_FS.with(|fs| {
                let mut fs_map = fs.lock().unwrap();
                for path in paths {
                    fs_map.insert(path, "".to_owned());
                }
            })
        }

        pub(crate) fn set_test_fs_path_content(path: &'static str, contents: String) {
            MOCK_FS.with(|fs| {
                let mut fs_map = fs.lock().unwrap();
                fs_map.insert(path, contents);
            })
        }

        fn with_test_fs<F, R>(callback: F) -> R
        where
            F: Fn(&HashMap<&'static str, String>) -> R
        {
            MOCK_FS.with(|fs| callback(&fs.lock().unwrap()))
        }

        pub(crate) fn is_path_file(path: &PathBuf) -> bool {
            with_test_fs(|fs| {
                fs.contains_key(&path.to_str().unwrap())
            })
        }

        pub(crate) fn read_file_to_string(path: &Path) -> std::io::Result<String> {
            with_test_fs(|fs| {
                match fs.get(&path.to_str().unwrap()) {
                    Some(contents) => Ok(contents.clone()),
                    None           => Err(std::io::Error::new(std::io::ErrorKind::NotFound,
                        format!("Test filesystem path {:?} does not exist", path)))
                }
            })
        }

        // ---------------------------------------------------------------------
        // End: test virtual filesystem
        // ---------------------------------------------------------------------
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fname_from_path_returns_file_name_even_if_file_does_not_exist() {
        assert_eq!("some_name", fname_from_path(Path::new("some_name")));
        assert_eq!("some_name", fname_from_path(Path::new("/some_name")));
        assert_eq!("some_name", fname_from_path(Path::new("/a/b/some_name")));
    }

    #[test]
    fn fname_from_path_returns_file_name_even_if_it_looks_like_a_directory() {
        assert_eq!("some_name", fname_from_path(Path::new("some_name/")));
    }

    #[test]
    #[should_panic]
    fn fname_from_path_panics_when_path_is_empty() {
        assert_eq!("", fname_from_path(Path::new("")));
    }

    #[test]
    #[should_panic]
    fn fname_from_path_panics_when_path_has_no_filename() {
        assert_eq!("", fname_from_path(Path::new("/a/")));
    }

    #[test]
    fn map_macro() {
        let mut one = std::collections::HashMap::new();
        one.insert(1, 'a');
        assert_eq!(one, map!{ 1 => 'a' });

        let mut two = std::collections::HashMap::new();
        two.insert("a", 1);
        two.insert("b", 2);
        assert_eq!(two, map!{ "a" => 1, "b" => 2 });
    }

    #[test]
    fn btreeset_join() {
        let empty: BTreeSet<String> = vec![].into_iter().collect();
        assert_eq!("", empty.join(""));
        assert_eq!("", empty.join(","));

        let one: BTreeSet<String> = vec!["a"].into_iter().map(|s| s.to_owned()).collect();
        assert_eq!("a", one.join(""));
        assert_eq!("a", one.join(","));

        let two: BTreeSet<String> = vec!["a", "b"].into_iter().map(|s| s.to_owned()).collect();
        assert_eq!("ab", two.join(""));
        assert_eq!("a,b", two.join(","));
    }
}