# gitaly-repo-rebuilder
Locates, validates, and reconstructs bare Git repositories from Gitaly storage, including automatic cleanup of empty and duplicate repos.
# Usage
#### Edit these constants
```rust
// Path to the Gitaly hashed file system that you want to recover
// Is ok to leave empty
// If you need more, add to the "roots" array
const PATH_TO_REPOS_ONE: &str = "/path/to/gitaly/repositories";
const PATH_TO_REPOS_TWO: &str = "";
const PATH_TO_REPOS_THREE: &str = "";

// The recovered repositories will be saved in this directory
const PATH_TO_RECOVERED_REPOS: &str = "/recovered/repos/get/put/here";

// Empty repositories will be saved in this directory
// Empty means there is a .git file but nothing else 
const PATH_TO_EMPTY_RECOVERED_REPOS: &str = "/empty/repos/go/here";

// Duplicate repositories will be saved in this directory
// Duplicates are determined by hashing Git metadata and comparing fingerprints   
const PATH_TO_DUPLICATE_RECOVERED_REPOS: &str = "/duplicate/repos/go/here";
```   
