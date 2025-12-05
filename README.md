# gitaly-repo-rebuilder
Locates, validates, and reconstructs bare Git repositories from Gitaly storage, including automatic cleanup of empty and duplicate repos.



// Duplicates are determined by hashing git data and comparison    
// Empty means there is a .git file but nothing else    