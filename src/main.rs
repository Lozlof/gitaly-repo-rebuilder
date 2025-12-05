//! -- // gitaly-repo-rebuilder // -- ///

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use std::hash::{Hasher, Hash};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;

const PATH_TO_REPOS_ONE: &str = "/path/to/gitaly/repositories";
const PATH_TO_REPOS_TWO: &str = "";
const PATH_TO_REPOS_THREE: &str = "";
const PATH_TO_RECOVERED_REPOS: &str = "/recovered/repos/get/put/here";
const PATH_TO_EMPTY_RECOVERED_REPOS: &str = "/empty/repos/go/here"; 
const PATH_TO_DUPLICATE_RECOVERED_REPOS: &str = "/duplicate/repos/go/here";

fn main() {
    let roots = [
        PATH_TO_REPOS_ONE,
        PATH_TO_REPOS_TWO,
        PATH_TO_REPOS_THREE,
    ];

    for root in &roots {
        if root.trim().is_empty() {
            println!("Skipping empty root entry");
            continue;
        }
    
        if let Err(err) = process_gitaly_root(root) {
            eprintln!("ERROR processing {}: {:?}", root, err);
            exit(1);
        }

        let recovered_count = match count_directories(Path::new(PATH_TO_RECOVERED_REPOS)) {
            Ok(count) => count,
            Err(err) => {
                eprintln!("{:?}", err);
                exit(1); 
            }
        };
        let empty_count = match count_directories(Path::new(PATH_TO_EMPTY_RECOVERED_REPOS)) {
            Ok(count) => count,
            Err(err) => {
                eprintln!("{:?}", err);
                exit(1); 
            }
        };

        println!("Finished {}", root);
        println!("Recovered count: {}", recovered_count);
        println!("Empty count: {}", empty_count);
    }

    if let Err(err) = move_duplicate_recovered_repos() {
        eprintln!("ERROR detecting duplicates: {:?}", err);
        exit(1);
    }

    let recovered_count = match count_directories(Path::new(PATH_TO_RECOVERED_REPOS)) {
        Ok(count) => count,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1); 
        }
    };
    let empty_count = match count_directories(Path::new(PATH_TO_EMPTY_RECOVERED_REPOS)) {
        Ok(count) => count,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1); 
        }
    };
    let duplicate_count = match count_directories(Path::new(PATH_TO_DUPLICATE_RECOVERED_REPOS)) {
        Ok(count) => count,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1); 
        }
    };

    println!("After dedupe:");
    println!("Recovered count: {}", recovered_count);
    println!("Empty count: {}", empty_count);
    println!("Duplicate count: {}", duplicate_count);

    println!("ALL DONE");
    exit(0);
}

fn process_gitaly_root(root_path: &str) -> Result<(), String> {
    let root = Path::new(root_path);

    if !root.is_dir() {
        return Err(format!("ND00: {}", root.display()));
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut confirmed: Vec<PathBuf> = Vec::new();

    find_candidate_repos(root, &mut candidates)?;

    for path in &candidates {
        match validate_bare_repo_with_git(path) {
            Ok(()) => confirmed.push(path.clone()),
            Err(err) => return Err(format!("VALIDATE_FAIL: {:?} :: {:?}", path, err)),
        }
    }

    recover_repos(&confirmed)?;
    move_empty_recovered_repos()?;

    println!("Done processing {} -> candidates={}, confirmed={}", root_path, candidates.len(), confirmed.len());

    return Ok(());
}

fn find_candidate_repos(dir: &Path, candidates: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) =>return Err(format!("0AA0: {:?}", err)),
    };

    for entry_result in entries {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) => return Err(format!("1BB1: {:?}", err)),
        };

        let path = entry.path();

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(err) => return Err(format!("2CC2: {:?}", err)),
        };

        if metadata.is_dir() {
            if looks_like_bare_repo(&path) {
                candidates.push(path.clone());
            }

            find_candidate_repos(&path, candidates)?;
        }
    }    

    return Ok(());
}

fn looks_like_bare_repo(path: &Path) -> bool {
    let head = path.join("HEAD");
    let config = path.join("config");
    let objects = path.join("objects");
    let refs = path.join("refs");

    return head.is_file() && config.is_file() && objects.is_dir() && refs.is_dir();
}

fn validate_bare_repo_with_git(path: &Path) -> Result<(), String> {
    let output = {
        match Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--is-bare-repository")
        .output() {
            Ok(o) => o,
            Err(err) => return Err(format!("M00M: {:?}", err)),
        }
    };

    if !output.status.success() {
        return Err(format!("V11V{:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let answer = stdout.trim();

    if answer == "true" {
        Ok(())
    } else {
        Err(format!("S22S: {:?})", answer))
    }
}

fn generate_random_repo_name() -> Result<String, String> {
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_nanos(),
        Err(error) => return Err(format!("9KKK9: {:?}", error)),
    };

    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);

    let hash_value = hasher.finish();

    return Ok(format!("repo-{:x}", hash_value));
}

fn recover_repos(confirmed: &Vec<PathBuf>) -> Result<(), String> {
    let recover_root = Path::new(PATH_TO_RECOVERED_REPOS);

    if recover_root.exists() {
        if !recover_root.is_dir() {
            return Err(format!("R01R: {}", recover_root.display()));
        }
    } else {
        if let Err(err) = fs::create_dir_all(recover_root) {
            return Err(format!("R02R: {}: {:?}", recover_root.display(), err));
        }
    }

    for repo_path in confirmed {
        let repo_name = match generate_random_repo_name() {
            Ok(name) => name,
            Err(err) => return Err(format!("G05G: {:?}", err))
        };

        let dest = recover_root.join(&repo_name);

        if dest.exists() {
            return Err("EX1EX".to_string());
        }

        println!("Recovering repo: {} -> {}", repo_path.display(), dest.display());

        let output = match Command::new("git")
            .arg("clone")
            .arg(repo_path)
            .arg(&dest)
            .output()
        {
            Ok(o) => o,
            Err(err) => return Err(format!("CL99CL: {:?}", err)),
        };

        if output.status.success() {
            println!("success");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("FA54FA: {:?}", stderr));
        }
    }

    return Ok(()); 
}

fn move_empty_recovered_repos() -> Result<(), String> {
    let recovered_root = Path::new(PATH_TO_RECOVERED_REPOS);
    let empty_root = Path::new(PATH_TO_EMPTY_RECOVERED_REPOS);
    let duplicate_root = Path::new(PATH_TO_DUPLICATE_RECOVERED_REPOS);

    if !empty_root.exists() {
        if let Err(err) = fs::create_dir_all(empty_root) {
            return Err(format!("E0S0E: {:?}", err));
        }
    }

    let entries = match fs::read_dir(recovered_root) {
        Ok(e) => e,
        Err(err) => return Err(format!("E1SS1E: {:?}", err)),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => return Err(format!("E1AA2E: {:?}", err)),
        };

        let path = entry.path();

        if !path.is_dir() {
            return Err(format!("ENN22E"))
        }

        if path == empty_root {
            continue;
        }
        if path == duplicate_root {
            continue;
        }

        let mut file_count = 0usize;
        let mut has_git = false;

        let inner = match fs::read_dir(&path) {
            Ok(i) => i,
            Err(err) => return Err(format!("E13E: {:?} :: {:?}", path, err)),
        };

        for item in inner {
            let item = match item {
                Ok(i) => i,
                Err(err) => return Err(format!("E1734E: {:?}", err)),
            };

            let name = item.file_name();
            let name_str = name.to_string_lossy();

            file_count += 1;

            if name_str == ".git" {
                has_git = true;
            }
        }

        if has_git && file_count == 1 {
            let folder_name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => return Err("E1522E".to_string()),
            };

            let new_path = empty_root.join(folder_name);

            println!("Moving EMPTY repo: {} -> {}", path.display(), new_path.display());

            if let Err(err) = fs::rename(&path, &new_path) {
                return Err(format!("E122S: {:?}", err));
            }
        }
    }

    return Ok(());
}

fn count_directories(path: &Path) -> Result<usize, String> {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(err) => return Err(format!("C0C0: {:?} :: {:?}", path, err)),
    };

    let mut count = 0usize;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => return Err(format!("C1C1: {:?}", err)),
        };

        let p = entry.path();
        if p.is_dir() {
            count += 1;
        }
    }

    return Ok(count);
}

fn compute_git_fingerprint(repo_path: &Path) -> Result<u64, String> {
    if !repo_path.is_dir() {
        return Err(format!("GF0: not dir: {}", repo_path.display()));
    }

    println!("  [GF] Computing git fingerprint for {}", repo_path.display());

    let show_ref = match Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("show-ref")
        .arg("--heads")
        .arg("--tags")
        .output()
    {
        Ok(o) => o,
        Err(err) => return Err(format!("GF1: failed show-ref for {} :: {:?}", repo_path.display(), err)),
    };

    let mut hasher = DefaultHasher::new();

    if show_ref.status.success() {
        let stdout = String::from_utf8_lossy(&show_ref.stdout);
        let mut lines: Vec<&str> = stdout.lines().collect();
        lines.sort();
        for line in &lines {
            line.hash(&mut hasher);
        }
    } else {
        let head = match Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
        {
            Ok(o) => o,
            Err(err) => return Err(format!("GF2: failed rev-parse HEAD for {} :: {:?}", repo_path.display(), err)),
        };

        if head.status.success() {
            let stdout = String::from_utf8_lossy(&head.stdout);
            stdout.trim().hash(&mut hasher);
        } else {
            let stderr = String::from_utf8_lossy(&head.stderr);
            stderr.hash(&mut hasher);
        }
    }

    let origin = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output();

    if let Ok(o) = origin {
        if o.status.success() {
            let url = String::from_utf8_lossy(&o.stdout);
            url.trim().hash(&mut hasher);
        }
    }

    let fingerprint = hasher.finish();
    println!("  [GF] Done git fingerprint for {} -> {:016x}", repo_path.display(), fingerprint);

    return Ok(fingerprint);
}

fn move_duplicate_recovered_repos() -> Result<(), String> {
    let recovered_root = Path::new(PATH_TO_RECOVERED_REPOS);
    let empty_root = Path::new(PATH_TO_EMPTY_RECOVERED_REPOS);
    let duplicate_root = Path::new(PATH_TO_DUPLICATE_RECOVERED_REPOS);

    if !duplicate_root.exists() {
        if let Err(err) = fs::create_dir_all(duplicate_root) {
            return Err(format!("D0D0: {:?}", err));
        }
    }

    println!("Starting duplicate scan in {}", recovered_root.display());

    let entries = match fs::read_dir(recovered_root) {
        Ok(e) => e,
        Err(err) => return Err(format!("D1D1: {:?}", err)),
    };

    let mut seen: HashMap<u64, PathBuf> = HashMap::new();
    let mut duplicate_count: usize = 0;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => return Err(format!("D1A2: {:?}", err)),
        };

        let path = entry.path();

        if !path.is_dir() {
            continue;
        }
        if path == empty_root {
            println!("  [DD] Skipping EMPTY root: {}", path.display());
            continue;
        }
        if path == duplicate_root {
            println!("  [DD] Skipping DUPLICATES root: {}", path.display());
            continue;
        }

        println!("[DD] Scanning repo: {}", path.display());

        let fingerprint = match compute_git_fingerprint(&path) {
            Ok(f) => f,
            Err(err) => return Err(format!("D2D2: {:?} :: {:?}", path, err)),
        };

        println!(
            "  [DD] Fingerprint for {}: {:016x}",
            path.display(),
            fingerprint
        );

        if let Some(original) = seen.get(&fingerprint) {
            let folder_name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => return Err("D3D3: invalid folder name".to_string()),
            };

            let new_path = duplicate_root.join(folder_name);
            println!(
                "  [DD] Moving DUPLICATE repo: {} (duplicate of {}) -> {}",
                path.display(),
                original.display(),
                new_path.display()
            );

            if let Err(err) = fs::rename(&path, &new_path) {
                return Err(format!("D4D4: {:?}", err));
            }

            duplicate_count += 1;
        } else {
            println!("  [DD] First time seeing this fingerprint, keeping {}", path.display());
            seen.insert(fingerprint, path.clone());
        }
    }

    println!(
        "Finished duplicate scan: unique repos kept = {}, duplicates moved = {}",
        seen.len(),
        duplicate_count
    );

    return Ok(());
}

// --- // gitaly-repo-rebuilder // --- //