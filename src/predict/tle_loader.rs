use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use sgp4::{Constants, Elements};

use crate::predict::error::PredictError;
use crate::predict::types::SatelliteInfo;

pub struct TleEntry {
    pub info: SatelliteInfo,
    pub elements: Elements,
    pub constants: Constants,
}

pub struct TleLoader {
    tle_dir: PathBuf,
    satellites: HashMap<u32, TleEntry>,
}

impl TleLoader {
    pub fn new(tle_dir: PathBuf) -> Self {
        Self {
            tle_dir,
            satellites: HashMap::new(),
        }
    }

    /// Load all TLE files from the directory
    pub fn load_all(&mut self) -> Result<(), PredictError> {
        if !self.tle_dir.exists() {
            return Err(PredictError::DirectoryNotFound(
                self.tle_dir.display().to_string(),
            ));
        }

        self.satellites.clear();

        let entries = fs::read_dir(&self.tle_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "tle" || ext == "txt" {
                        match self.parse_tle_file(&path) {
                            Ok(entries) => {
                                for tle_entry in entries {
                                    self.satellites.insert(tle_entry.info.norad_id, tle_entry);
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to parse TLE file {}: {}", path.display(), e);
                                // Continue with other files
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse a single TLE file (may contain multiple satellites)
    fn parse_tle_file(&self, path: &Path) -> Result<Vec<TleEntry>, PredictError> {
        let content = fs::read_to_string(path)?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        let tle_entries = parse_multi_tle(&content);
        let mut results = Vec::new();

        for (name, line1, line2) in tle_entries {
            // Parse with sgp4
            let elements = match Elements::from_tle(
                name.clone(),
                line1.as_bytes(),
                line2.as_bytes(),
            ) {
                Ok(e) => e,
                Err(e) => {
                    return Err(PredictError::InvalidTle {
                        file: filename.clone(),
                        message: e.to_string(),
                    });
                }
            };

            let constants = match Constants::from_elements(&elements) {
                Ok(c) => c,
                Err(e) => {
                    return Err(PredictError::InvalidTle {
                        file: filename.clone(),
                        message: e.to_string(),
                    });
                }
            };

            let sat_name = name.unwrap_or_else(|| format!("NORAD {}", elements.norad_id));

            results.push(TleEntry {
                info: SatelliteInfo {
                    name: sat_name,
                    norad_id: elements.norad_id as u32,
                    tle_source: filename.clone(),
                },
                elements,
                constants,
            });
        }

        Ok(results)
    }

    /// Get all loaded satellites
    pub fn satellites(&self) -> Vec<&TleEntry> {
        self.satellites.values().collect()
    }

    /// Reload TLE files (called manually or by watcher)
    #[allow(dead_code)]
    pub fn reload(&mut self) -> Result<(), PredictError> {
        self.load_all()
    }
}

/// Parse multi-satellite TLE content
fn parse_multi_tle(content: &str) -> Vec<(Option<String>, String, String)> {
    let lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Check if current line is line1 (starts with "1 ")
        if lines[i].starts_with("1 ")
            && i + 1 < lines.len()
            && lines[i + 1].starts_with("2 ")
        {
            // 2-line TLE (no name)
            result.push((None, lines[i].to_string(), lines[i + 1].to_string()));
            i += 2;
        } else if i + 2 < lines.len()
            && lines[i + 1].starts_with("1 ")
            && lines[i + 2].starts_with("2 ")
        {
            // 3-line TLE (with name)
            result.push((
                Some(lines[i].to_string()),
                lines[i + 1].to_string(),
                lines[i + 2].to_string(),
            ));
            i += 3;
        } else {
            i += 1; // Skip unknown line
        }
    }

    result
}
