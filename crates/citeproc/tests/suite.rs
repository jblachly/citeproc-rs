// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2019 Corporation for Digital Scholarship

#![feature(custom_test_frameworks)]
#![test_runner(datatest::runner)]

use citeproc::{LocaleFetchError, LocaleFetcher, Processor};
use citeproc_io::output::{generic::MicroHtml, html::Html};
use citeproc_io::{
    Cite, CiteId, Cluster, ClusterId, Locator, NumericValue, Reference, Suppression,
};
use csl::locale::Lang;
use csl::terms::LocatorType;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use pretty_assertions::assert_eq;
use std::mem;
use std::str::FromStr;
use std::sync::Arc;

#[macro_use]
extern crate serde_derive;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
struct CitationItem {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    locator: Option<String>,
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    suffix: String,
    #[serde(default)]
    suppress_author: bool,
    #[serde(default)]
    author_only: bool,
}

impl CitationItem {
    fn to_cite(&self, n: u32) -> Cite<Html> {
        Cite {
            id: n,
            ref_id: csl::Atom::from(self.id.as_str()),
            prefix: MicroHtml(self.prefix.clone()),
            suffix: MicroHtml(self.suffix.clone()),
            locators: match (self.locator.as_ref(), self.label.as_ref()) {
                (Some(loc), Some(lab)) => vec![Locator(
                    LocatorType::from_str(&lab).expect("unknown locator type"),
                    NumericValue::from(std::borrow::Cow::from(loc)),
                )],
                _ => vec![],
            },
            // TODO: delete these
            locator_date: None,
            locator_extra: None,
            suppression: match (self.suppress_author, self.author_only) {
                (false, true) => Some(Suppression::InText),
                (true, false) => Some(Suppression::Rest),
                (false, false) => None,
                _ => panic!("multiple citation modes passed to CitationItem"),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
enum ResultKind {
    Dots,
    Arrows,
}
#[derive(Debug, PartialEq)]
struct CiteResult {
    kind: ResultKind,
    note_number: u32,
    text: String,
}
#[derive(Debug, PartialEq)]
struct Results(Vec<CiteResult>);

impl FromStr for Results {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use nom::{
            branch::alt,
            bytes::complete::{tag, take_until},
            character::complete::{char, digit1},
            combinator::map,
            multi::separated_nonempty_list,
            sequence::{delimited, preceded, tuple},
            IResult,
        };
        fn dots(inp: &str) -> IResult<&str, ResultKind> {
            map(alt((tag(".."), tag(">>"))), |s| match s {
                ".." => ResultKind::Dots,
                ">>" => ResultKind::Arrows,
                _ => unreachable!(),
            })(inp)
        }
        fn num(inp: &str) -> IResult<&str, u32> {
            map(delimited(char('['), digit1, char(']')), |ds: &str| {
                u32::from_str(ds).unwrap()
            })(inp)
        }
        fn formatted(inp: &str) -> IResult<&str, &str> {
            preceded(char(' '), take_until("\n"))(inp)
        }
        fn total(inp: &str) -> IResult<&str, CiteResult> {
            map(tuple((dots, num, formatted)), |(k, n, f)| CiteResult {
                kind: k,
                note_number: n,
                text: String::from(f),
            })(inp)
        }
        fn whole_thing(inp: &str) -> IResult<&str, Vec<CiteResult>> {
            separated_nonempty_list(char('\n'), total)(inp)
        }
        Ok(Results(whole_thing(s).unwrap().1))
    }
}

use serde::de::{Deserialize, Deserializer};

enum InstructionMode {
    Composite,
    AuthorOnly,
    SuppressAuthor,
}

impl<'de> Deserialize<'de> for InstructionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "author-only" => InstructionMode::AuthorOnly,
            "composite" => InstructionMode::Composite,
            "suppress-author" => InstructionMode::SuppressAuthor,
            _ => panic!("unrecognized instruction mode"),
        })
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "mode")]
enum ModeProperties {
    #[serde(rename = "composite")]
    Composite {
        #[serde(default)]
        infix: String,
    },
    #[serde(rename = "author-only")]
    AuthorOnly,
    #[serde(rename = "suppress-author")]
    SuppressAuthor,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct Properties {
    #[serde(default)]
    index: u32,
    note_index: u32,
    #[serde(default, flatten)]
    mode: Option<ModeProperties>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct CiteInstruction {
    #[serde(rename = "citationID")]
    cluster_id: String,
    #[serde(rename = "citationItems")]
    citation_items: Vec<CitationItem>,
    properties: Properties,
}
#[derive(Deserialize, Debug, PartialEq, Clone)]
struct PrePost(String, u32);
#[derive(Deserialize, Debug, PartialEq, Clone)]
struct Instruction(CiteInstruction, Vec<PrePost>, Vec<PrePost>);

use std::collections::HashMap;

struct JsExecutor<'a> {
    cluster_ids_mapping: HashMap<String, ClusterId>,
    current_note_numbers: HashMap<ClusterId, u32>,
    zeroes: Vec<ClusterId>,
    proc: &'a mut Processor,
    next_id: ClusterId,
    n_cite: CiteId,
}

impl JsExecutor<'_> {
    fn new<'a>(proc: &'a mut Processor) -> JsExecutor<'a> {
        JsExecutor {
            cluster_ids_mapping: HashMap::new(),
            current_note_numbers: HashMap::new(),
            proc,
            next_id: 1,
            n_cite: 1,
            zeroes: Vec::new(),
        }
    }
    fn get_id(&mut self, string_id: &str) -> ClusterId {
        if self.cluster_ids_mapping.contains_key(string_id) {
            return *self.cluster_ids_mapping.get(string_id).unwrap();
        } else {
            self.cluster_ids_mapping
                .insert(string_id.to_string(), self.next_id);
            let id = self.next_id;
            self.next_id += 1;
            return id;
        }
    }

    fn get_results(&self) -> Vec<CiteResult> {
        let updates = self.proc.batched_updates();
        let mut mod_clusters = HashMap::new();
        let mut results = Vec::<CiteResult>::new();
        for (id, text) in updates.clusters {
            mod_clusters.insert(id, true);
            let &note_number = self.current_note_numbers.get(&id).unwrap();
            let text = (*text).clone();
            results.push(CiteResult {
                kind: ResultKind::Arrows,
                note_number,
                text,
            })
        }
        for &id in self.current_note_numbers.keys() {
            if mod_clusters.contains_key(&id) {
                continue;
            }
            let &note_number = self.current_note_numbers.get(&id).unwrap();
            let text = (*self.proc.get_cluster(id)).clone();
            results.push(CiteResult {
                kind: ResultKind::Dots,
                note_number,
                text,
            })
        }
        results.sort_by_key(|x| x.note_number);
        results
    }

    fn format_results(&self) -> String {
        let results = self.get_results();
        let mut output = String::new();
        for (n, res) in results.iter().enumerate() {
            output.push_str(if res.kind == ResultKind::Arrows {
                ">>"
            } else {
                ".."
            });
            output.push_str("[");
            output.push_str(&format!(
                "{}",
                if res.note_number == 0 {
                    n as u32
                } else {
                    res.note_number
                }
            ));
            output.push_str("] ");
            output.push_str(&res.text);
            output.push_str("\n");
        }
        output
    }

    /// Note: this does not work very well. The way citeproc-js runs its own cannot easily be
    /// deciphered.
    fn execute(&mut self, instruction: &Instruction) {
        self.proc.drain();

        let Instruction(cite_i, _pre, _post) = instruction;
        let id = self.get_id(&*cite_i.cluster_id);
        let note_number = cite_i.properties.note_index;

        let mut cites = Vec::new();
        for cite_item in cite_i.citation_items.iter() {
            // TODO: technically this is not reusing (?) n_cite as it should be doing
            cites.push(cite_item.to_cite(self.n_cite));
            self.n_cite += 1;
        }
        let cluster = Cluster {
            id,
            note_number,
            cites,
        };

        let mut nonzero_cluster_ids: Vec<u32> = self
            .current_note_numbers
            .keys()
            .map(|&x| x)
            .filter(|&x| x != 0)
            .collect();

        if note_number == 0 {
            let ix = cite_i.properties.index as usize;
            if ix >= self.zeroes.len() {
                self.zeroes.push(id);
            } else {
                self.zeroes.insert(ix, id);
            }
            self.proc.replace_cluster(cluster);
        } else if self.current_note_numbers.contains_key(&id) {
            self.proc.replace_cluster(cluster);
        } else {
            let one_after = nonzero_cluster_ids
                .get(note_number as usize + 1)
                .map(|&x| x);
            nonzero_cluster_ids.insert(0, id);
            self.proc.insert_cluster(cluster, one_after);
        }
        self.current_note_numbers.insert(id, note_number);

        nonzero_cluster_ids.sort_by_key(|id| *self.current_note_numbers.get(id).unwrap());
        let mut renum = Vec::new();
        for (n, &id) in nonzero_cluster_ids.iter().enumerate() {
            renum.push(id);
            renum.push(n as u32 + 1);
            self.current_note_numbers.insert(id, n as u32 + 1);
        }
        self.proc.renumber_clusters(&renum);
    }
}

enum Chunk {
    // Required sections
    Mode(String),

    /// Interpretation depends on which mode you're using
    ///
    /// https://github.com/citation-style-language/test-suite#result
    Result(String),

    /// XML CSL style
    ///
    /// https://github.com/citation-style-language/test-suite#csl
    Csl(String),

    /// JSON Reference[] list
    ///
    /// https://github.com/citation-style-language/test-suite#input
    Input(String),

    // Optional sections
    /// JSON LIST of LISTS of bibliography entries as item IDs
    ///
    /// https://github.com/citation-style-language/test-suite#bibentries
    BibEntries(String),
    /// JSON input to bibliography mode for limiting bib output
    ///
    /// https://github.com/citation-style-language/test-suite#bibsection
    BibSection(String),
    /// JSON list of lists of cites (ie Cluster[].map(cl => cl.cites))
    ///
    /// https://github.com/citation-style-language/test-suite#citation-items
    CitationItems(String),
    /// JSON list of lists of objects that represent calls to processCitationCluster
    ///
    /// https://github.com/citation-style-language/test-suite#citations
    Citations(String),
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Mode {
    Citation,
    Bibliography,
}

#[derive(Debug, PartialEq)]
struct TestCase {
    mode: Mode,
    csl: String,
    input: Vec<Reference>,
    result: String,
    citation_items: Option<Vec<Vec<CitationItem>>>,
    citations: Option<Vec<Instruction>>,
}

fn parse_human_test(contents: &str) -> TestCase {
    use regex::Regex;
    lazy_static! {
        static ref BEGIN: Regex = Regex::new(r">>=+ ([A-Z\-]+) =+>>").unwrap();
    }
    lazy_static! {
        static ref END: Regex = Regex::new(r"<<=+ ([A-Z\-]+) =+<<").unwrap();
    }
    let mut state = None;
    let mut chunks = vec![];
    // some of the files use two or four equals signs, most use five.
    for line in contents.lines() {
        if END.is_match(line) {
            if state.is_some() {
                let mut chunk = None;
                mem::swap(&mut state, &mut chunk);
                chunks.push(chunk.unwrap());
            }
        } else if let Some(caps) = BEGIN.captures(line) {
            state = match caps.get(1).unwrap().as_str() {
                "MODE" => Some(Chunk::Mode(String::new())),
                "CSL" => Some(Chunk::Csl(String::new())),
                "INPUT" => Some(Chunk::Input(String::new())),
                "RESULT" => Some(Chunk::Result(String::new())),
                "BIBENTRIES" => Some(Chunk::BibEntries(String::new())),
                "BIBSECTION" => Some(Chunk::BibSection(String::new())),
                "CITATION-ITEMS" => Some(Chunk::CitationItems(String::new())),
                "CITATIONS" => Some(Chunk::Citations(String::new())),
                x => panic!("unrecognized block: {}", x),
            }
        } else {
            if let Some(ref mut state) = state {
                match state {
                    Chunk::Mode(ref mut s)
                    | Chunk::Csl(ref mut s)
                    | Chunk::Input(ref mut s)
                    | Chunk::Result(ref mut s)
                    | Chunk::BibSection(ref mut s)
                    | Chunk::BibEntries(ref mut s)
                    | Chunk::CitationItems(ref mut s)
                    | Chunk::Citations(ref mut s) => {
                        if !s.is_empty() {
                            s.push_str("\n");
                        }
                        s.push_str(line);
                    }
                }
            }
            // otherwise, it's a comment
        }
    }

    let mut mode = None;
    let mut csl = None;
    let mut input: Option<Vec<Reference>> = None;
    let mut result = None;

    // TODO
    let mut bib_entries = None;
    let mut bib_section = None;
    let mut citation_items = None;
    let mut citations = None;

    for chunk in chunks {
        match chunk {
            Chunk::Mode(m) => {
                mode = mode.or_else(|| match m.as_str() {
                    "citation" => Some(Mode::Citation),
                    "bibliography" => Some(Mode::Bibliography),
                    _ => panic!("unknown mode {}", m),
                })
            }
            Chunk::Csl(s) => csl = csl.or_else(|| Some(s)),
            Chunk::Input(s) => {
                input = input.or_else(|| {
                    Some(
                        serde_json::from_str(&s)
                            .expect("could not parse references in INPUT section"),
                    )
                })
            }
            Chunk::Result(s) => result = result.or_else(|| Some(s)),
            Chunk::BibEntries(s) => bib_entries = bib_entries.or_else(|| Some(s)),
            Chunk::BibSection(s) => bib_section = bib_section.or_else(|| Some(s)),
            Chunk::CitationItems(s) => {
                citation_items = citation_items.or_else(|| {
                    Some(serde_json::from_str(&s).expect("could not parse CITATION-ITEMS"))
                })
            }
            Chunk::Citations(s) => {
                citations = citations
                    .or_else(|| Some(serde_json::from_str(&s).expect("could not parse CITATIONS")))
            }
        }
    }
    TestCase {
        mode: mode.unwrap_or(Mode::Citation),
        input: input.expect("test case without an INPUT section"),
        result: result.expect("test case without a RESULT section"),
        csl: csl.expect("test case without a CSL section"),
        citation_items,
        citations,
    }
}

use std::path::PathBuf;

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

use std::fmt;

/// See https://github.com/colin-kiegel/rust-pretty-assertions/issues/24
///
/// Wrapper around string slice that makes debug output `{:?}` to print string same way as `{}`.
/// Used in different `assert*!` macros in combination with `pretty_assertions` crate to make
/// test failures to show nice diffs.
#[derive(PartialEq, Eq)]
#[doc(hidden)]
pub struct PrettyString<'a>(pub &'a str);

/// Make diff to display string as multi-line string
impl<'a> fmt::Debug for PrettyString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl TestCase {
    fn execute(mut self) {
        if self.mode == Mode::Bibliography {
            panic!("bib tests not implemented");
        }
        let fet = Arc::new(Filesystem::project_dirs());
        let mut proc = Processor::new(&self.csl, fet, true).expect("could not construct processor");
        let mut clusters: Vec<Cluster<Html>> = Vec::new();

        let mut res = String::new();
        if let Some(ref instructions) = &self.citations {
            self.result.push_str("\n");
            proc.set_references(self.input);
            let mut executor = JsExecutor::new(&mut proc);
            for instruction in instructions.iter() {
                executor.execute(instruction);
            }
            // let desired = Results::from_str(&self.result).unwrap();
            // turns out it's easier to just produce the string the same way
            res = executor.format_results();
        } else {
            if let Some(ref citation_items) = &self.citation_items {
                let mut n_cluster = 1u32;
                let mut n_cite = 1u32;
                for clus in citation_items {
                    let mut cites = Vec::new();
                    for cite_item in clus.iter() {
                        cites.push(cite_item.to_cite(n_cite));
                        n_cite += 1;
                    }
                    clusters.push(Cluster {
                        id: n_cluster,
                        note_number: n_cluster,
                        cites,
                    });
                    n_cluster += 1;
                }
            } else {
                let mut cites = Vec::new();
                // TODO: assemble cites/clusters the other few available ways
                for (n, refr) in self.input.iter().enumerate() {
                    let n = n as u32;
                    cites.push(Cite::basic(n, &refr.id));
                }
                clusters.push(Cluster {
                    id: 1,
                    note_number: 1,
                    cites,
                });
            }

            proc.set_references(self.input);
            proc.init_clusters(clusters.clone());
            let mut pushed = false;
            for cluster in clusters.iter() {
                let html = proc.get_cluster(cluster.id);
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
        assert_eq!(PrettyString(&res), PrettyString(&self.result))
    }
}

use std::collections::HashSet;
use std::fs::read_to_string;
use std::path::Path;

lazy_static! {
    static ref BIB_TESTS: HashSet<String> = {
        let mut m = HashSet::new();
        // cargo test -- 2>/dev/null | rg 'bib tests' |  rg suite | cut -d' ' -f2 | cut -d: -f3 | cut -d\' -f1 > bibtests.txt
        let bibtests = include_str!("./data/bibtests.txt");
        for bibtest in bibtests.lines() {
            m.insert(bibtest.to_string());
        }
        m
    };
}

fn is_ignore(path: &Path) -> bool {
    let fname = path.file_name().unwrap().to_string_lossy();
    BIB_TESTS.contains(&fname.into_owned())
}

#[datatest::files("tests/data/test-suite/processor-tests/humans", {
    path in r"^(.*)\.txt" if !is_ignore,
})]
fn suite_case(path: &Path) {
    let input = read_to_string(path).unwrap();
    let test_case = parse_human_test(&input);
    test_case.execute();
}
