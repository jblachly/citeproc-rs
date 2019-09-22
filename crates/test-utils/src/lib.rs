// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2019 Corporation for Digital Scholarship

#[macro_use]
extern crate serde_derive;

use citeproc::prelude::*;
use citeproc_io::{
    Cite, Cluster2, ClusterId, ClusterNumber, IntraNote, Locator, NumericValue, Reference,
    Suppression,
};
use csl::locale::Lang;
use csl::terms::LocatorType;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer};
use std::mem;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

pub mod humans;
// pub mod toml;
pub mod yaml;

use humans::{CiteprocJsInstruction, JsExecutor};

#[derive(Deserialize, Debug, PartialEq)]
pub struct TestCase {
    pub mode: Mode,
    #[serde(default)]
    pub format: Format,
    pub csl: String,
    pub input: Vec<Reference>,
    pub result: String,
    pub clusters: Option<Vec<Cluster2<Markup>>>,
    pub process_citation_clusters: Option<Vec<CiteprocJsInstruction>>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mode {
    Citation,
    Bibliography,
}
impl Default for Mode {
    fn default() -> Self {
        Mode::Citation
    }
}

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "citation" => Mode::Citation,
            "bibliography" => Mode::Bibliography,
            _ => panic!("unrecognized test mode"),
        })
    }
}

#[derive(Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct Format(SupportedFormat);
impl Default for Format {
    fn default() -> Self {
        Format(SupportedFormat::TestHtml)
    }
}

impl TestCase {
    pub fn execute(&mut self) -> String {
        if self.mode == Mode::Bibliography {
            panic!("bib tests not implemented");
        }
        let fet = Arc::new(Filesystem::project_dirs());
        let mut proc = Processor::new(&self.csl, fet, true, self.format.0)
            .expect("could not construct processor");

        let mut res = String::new();
        if let Some(ref instructions) = &self.process_citation_clusters {
            self.result.push_str("\n");
            proc.set_references(self.input.clone());
            let mut executor = JsExecutor::new(&mut proc);
            for instruction in instructions.iter() {
                executor.execute(instruction);
            }
            // let desired = Results::from_str(&self.result).unwrap();
            // turns out it's easier to just produce the string the same way
            res = executor.format_results();
        } else {
            let mut clusters_auto = Vec::new();
            let clusters = if let Some(ref clusters) = &self.clusters {
                clusters
            } else {
                let mut cites = Vec::new();
                // TODO: assemble cites/clusters the other few available ways
                for refr in self.input.iter() {
                    cites.push(Cite::basic(&refr.id));
                }
                clusters_auto.push(Cluster2::Note {
                    id: 1,
                    note: IntraNote::Single(1),
                    cites,
                });
                &clusters_auto
            };

            proc.set_references(self.input.clone());
            proc.init_clusters(clusters.clone());
            let mut pushed = false;
            for cluster in clusters.iter() {
                let html = proc.get_cluster(cluster.id());
                if pushed {
                    res.push_str("\n");
                }
                res.push_str(&*html);
                pushed = true;
            }
        }
        if self.result == "[CSL STYLE ERROR: reference with no printed form.]" {
            self.result = String::new()
        }
        // Because citeproc-rs is a bit keen to escape things
        // Slashes are fine if they're not next to angle braces
        // let's hope they're not
        res.replace("&#x2f;", "/")
    }
}

struct Filesystem {
    root: PathBuf,
}

impl Filesystem {
    fn new(repo_dir: impl Into<PathBuf>) -> Self {
        Filesystem {
            root: repo_dir.into(),
        }
    }
    fn project_dirs() -> Self {
        let pd = ProjectDirs::from("net", "cormacrelf", "citeproc-rs")
            .expect("No home directory found.");
        let mut locales_dir = pd.cache_dir().to_owned();
        locales_dir.push("locales");
        Self::new(locales_dir)
    }
}

use std::{fs, io};

impl LocaleFetcher for Filesystem {
    fn fetch_string(&self, lang: &Lang) -> Result<Option<String>, LocaleFetchError> {
        let mut path = self.root.clone();
        path.push(&format!("locales-{}.xml", lang));
        let read = fs::read_to_string(path);
        match read {
            Ok(string) => Ok(Some(string)),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => Ok(None),
                _ => Err(LocaleFetchError::Io(e)),
            },
        }
    }
}
