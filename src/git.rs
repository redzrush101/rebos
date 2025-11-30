use std::process::Command;
use std::io;
use piglog::prelude::*;
use piglog::*;
use crate::places;

pub struct GitRepo {
    path: String,
}

impl GitRepo {
    pub fn new() -> Self {
        Self {
            path: places::base().display().to_string(),
        }
    }

    fn run_git_command(&self, args: &[&str]) -> Result<String, io::Error> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Git command failed: {}", stderr);
                    Err(io::Error::new(io::ErrorKind::Other, stderr.to_string()))
                }
            }
            Err(e) => {
                error!("Failed to execute git command: {}", e);
                Err(e)
            }
        }
    }

    pub fn init_if_needed(&self) -> Result<(), io::Error> {
        if !places::base().exists() {
            error!("Rebos base directory does not exist!");
            return Err(io::Error::new(io::ErrorKind::NotFound, "Base directory not found"));
        }

        let git_dir = places::base().join(".git");
        
        if !git_dir.exists() {
            info!("Initializing Git repository...");
            self.run_git_command(&["init"])?;
            
            // Configure git user if not set
            if self.run_git_command(&["config", "user.name"]).is_err() {
                self.run_git_command(&["config", "user.name", "Rebos"])?;
            }
            if self.run_git_command(&["config", "user.email"]).is_err() {
                self.run_git_command(&["config", "user.email", "rebos@localhost"])?;
            }

            // Create .gitignore
            let gitignore_path = places::base().join(".gitignore");
            std::fs::write(&gitignore_path, "lock\n")?;
            self.run_git_command(&["add", ".gitignore"])?;
            self.run_git_command(&["commit", "-m", "Initial commit"])?;
            
            success!("Git repository initialized");
        }

        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<String, io::Error> {
        self.init_if_needed()?;
        
        // Add all changes
        self.run_git_command(&["add", "."])?;
        
        // Check if there are changes to commit
        let status = self.run_git_command(&["status", "--porcelain"])?;
        if status.trim().is_empty() {
            warning!("No changes to commit");
            return Ok(String::new());
        }

        // Commit changes
        self.run_git_command(&["commit", "-m", message])?;
        
        // Get commit hash
        let hash = self.get_current_hash()?;
        success!("Committed generation: {}", hash);
        
        Ok(hash)
    }

    pub fn get_current_hash(&self) -> Result<String, io::Error> {
        self.run_git_command(&["rev-parse", "HEAD"])
    }

    pub fn get_file_content_at_hash(&self, hash: &str, file_path: &str) -> Result<String, io::Error> {
        self.run_git_command(&["show", &format!("{}:{}", hash, file_path)])
    }

    pub fn log(&self, limit: Option<usize>) -> Result<Vec<(String, String)>, io::Error> {
        let output = if let Some(limit) = limit {
            self.run_git_command(&["log", "--pretty=format:%H|%s", &format!("-{}", limit)])?
        } else {
            self.run_git_command(&["log", "--pretty=format:%H|%s"])?
        };
        
        let mut commits = Vec::new();
        for line in output.lines() {
            if let Some((hash, message)) = line.split_once('|') {
                commits.push((hash.to_string(), message.to_string()));
            }
        }

        Ok(commits)
    }

    pub fn checkout(&self, hash: &str) -> Result<(), io::Error> {
        // Stash any changes before checkout
        if self.is_dirty()? {
            self.run_git_command(&["stash", "push", "-m", "Auto-stash before rollback"])?;
        }
        
        self.run_git_command(&["checkout", hash])?;
        success!("Checked out generation: {}", hash);
        Ok(())
    }

    pub fn get_diff(&self, from_hash: &str, to_hash: &str) -> Result<String, io::Error> {
        self.run_git_command(&["diff", &format!("{}..{}", from_hash, to_hash)])
    }

    pub fn is_dirty(&self) -> Result<bool, io::Error> {
        let status = self.run_git_command(&["status", "--porcelain"])?;
        Ok(!status.trim().is_empty())
    }
}

// Global Git repository instance
pub fn repo() -> GitRepo {
    GitRepo::new()
}