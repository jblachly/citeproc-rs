// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2019 Corporation for Digital Scholarship

pub mod update;

#[cfg(test)]
mod test;

use crate::prelude::*;

use self::update::{DocUpdate, UpdateSummary};
use citeproc_db::{CiteDatabaseStorage, HasFetcher, LocaleDatabaseStorage, StyleDatabaseStorage};
use citeproc_proc::db::IrDatabaseStorage;

use parking_lot::Mutex;
use salsa::Durability;
#[cfg(feature = "rayon")]
use salsa::{ParallelDatabase, Snapshot};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use csl::error::StyleError;
use csl::locale::Lang;
use csl::style::Style;

use citeproc_io::output::{markup::Markup, OutputFormat};
use citeproc_io::{Cite, Cluster2, ClusterId, ClusterNumber, Reference};
use csl::Atom;

#[salsa::database(
    StyleDatabaseStorage,
    LocaleDatabaseStorage,
    CiteDatabaseStorage,
    IrDatabaseStorage
)]
pub struct Processor {
    runtime: salsa::Runtime<Self>,
    pub fetcher: Arc<dyn LocaleFetcher>,
    queue: Arc<Mutex<Vec<DocUpdate>>>,
    save_updates: bool,
    formatter: Markup,
}

/// This impl tells salsa where to find the salsa runtime.
impl salsa::Database for Processor {
    fn salsa_runtime(&self) -> &salsa::Runtime<Processor> {
        &self.runtime
    }

    /// A way to extract imperative update sequences from a "here's the entire world" API. An
    /// editor might require simple instructions to update a document; modify this footnote,
    /// replace that bibliography entry. We will use Salsa WillExecute events to determine which
    /// things were recomputed, and assume a recomputation means re-rendering is necessary.
    fn salsa_event(&self, event_fn: impl Fn() -> salsa::Event<Self>) {
        if !self.save_updates {
            return;
        }
        use self::__SalsaDatabaseKeyKind::IrDatabaseStorage as RDS;
        use citeproc_proc::db::IrDatabaseGroupKey__ as GroupKey;
        use salsa::EventKind::*;
        let mut q = self.queue.lock();
        match event_fn().kind {
            WillExecute { database_key } => match database_key.kind {
                RDS(GroupKey::built_cluster(key)) => {
                    let upd = DocUpdate::Cluster(key);
                    // info!("produced update, {:?}", upd);
                    q.push(upd)
                }
                _ => {}
            },
            _ => {}
        };
    }
}

#[cfg(feature = "rayon")]
impl ParallelDatabase for Processor {
    fn snapshot(&self) -> Snapshot<Self> {
        Snapshot::new(Processor {
            runtime: self.runtime.snapshot(self),
            fetcher: self.fetcher.clone(),
            queue: self.queue.clone(),
            save_updates: self.save_updates,
            formatter: self.formatter.clone(),
        })
    }
}

impl HasFetcher for Processor {
    fn get_fetcher(&self) -> Arc<dyn LocaleFetcher> {
        self.fetcher.clone()
    }
}

impl HasFormatter for Processor {
    fn get_formatter(&self) -> Markup {
        self.formatter.clone()
    }
}

// need a Clone impl for map_with
// thanks to rust-analyzer for the tip
#[cfg(feature = "rayon")]
struct Snap(pub salsa::Snapshot<Processor>);
#[cfg(feature = "rayon")]
impl Clone for Snap {
    fn clone(&self) -> Self {
        Snap(self.0.snapshot())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SupportedFormat {
    Html,
    Rtf,
    #[cfg(feature = "test")]
    TestHtml,
}

impl FromStr for SupportedFormat {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(SupportedFormat::Html),
            "rtf" => Ok(SupportedFormat::Rtf),
            _ => Err(()),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for SupportedFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        use serde::de::Error as DeError;
        SupportedFormat::from_str(s.as_str())
            .map_err(|()| DeError::custom(format!("unknown format {}", s.as_str())))
    }
}

impl Processor {
    pub(crate) fn safe_default(fetcher: Arc<dyn LocaleFetcher>) -> Self {
        let mut db = Processor {
            runtime: Default::default(),
            fetcher,
            queue: Arc::new(Mutex::new(Default::default())),
            save_updates: false,
            formatter: Markup::default(),
        };
        citeproc_db::safe_default(&mut db);
        db
    }

    pub fn new(
        style_string: &str,
        fetcher: Arc<dyn LocaleFetcher>,
        save_updates: bool,
        format: SupportedFormat,
    ) -> Result<Self, StyleError> {
        let mut db = Processor::safe_default(fetcher);
        db.save_updates = save_updates;
        db.formatter = match format {
            SupportedFormat::Html => Markup::html(),
            SupportedFormat::Rtf => Markup::rtf(),
            #[cfg(feature = "test")]
            SupportedFormat::TestHtml => {
                use citeproc_io::output::markup::HtmlOptions;
                Markup::Html(HtmlOptions::test_suite())
            }
        };
        let style = Arc::new(Style::from_str(style_string)?);
        db.set_style_with_durability(style, Durability::MEDIUM);
        Ok(db)
    }

    pub fn set_style_text(&mut self, style_text: &str) -> Result<(), StyleError> {
        let style = Style::from_str(style_text)?;
        self.set_style_with_durability(Arc::new(style), Durability::MEDIUM);
        Ok(())
    }

    #[cfg(test)]
    pub fn test_db() -> Self {
        use citeproc_db::PredefinedLocales;
        Processor::safe_default(Arc::new(PredefinedLocales(Default::default())))
    }

    #[cfg(feature = "rayon")]
    fn snap(&self) -> Snap {
        Snap(self.snapshot())
    }

    // TODO: This might not play extremely well with Salsa's garbage collector,
    // which will have a new revision number for each built_cluster call.
    // Probably better to have this as a real query.
    pub fn compute(&self) {
        let cluster_ids = self.cluster_ids();
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            let cite_ids = self.all_cite_ids();
            // compute ir2s, so the first year_suffixes call doesn't trigger all ir2s on a
            // single rayon thread
            cite_ids
                .par_iter()
                .for_each_with(self.snap(), |snap, &cite_id| {
                    snap.0.ir_gen2_add_given_name(cite_id);
                });
            self.year_suffixes();
            cluster_ids
                .par_iter()
                .for_each_with(self.snap(), |snap, &cluster_id| {
                    snap.0.built_cluster(cluster_id);
                });
        }
        #[cfg(not(feature = "rayon"))]
        {
            for &cluster_id in cluster_ids.iter() {
                self.built_cluster(cluster_id);
            }
        }
    }

    pub fn batched_updates(&self) -> UpdateSummary {
        if !self.save_updates {
            return UpdateSummary::default();
        }
        self.compute();
        let mut queue = self.queue.lock();
        let summary = UpdateSummary::summarize(self, &*queue);
        queue.clear();
        summary
    }

    pub fn drain(&mut self) {
        self.compute();
        let mut queue = self.queue.lock();
        queue.clear();
    }

    // // TODO: make this use a function exported from citeproc_proc
    // pub fn single(&self, ref_id: &Atom) -> <Markup as OutputFormat>::Output {
    //     let fmt = Markup::default();
    //     let refr = match self.reference(ref_id.clone()) {
    //         None => return fmt.output(fmt.plain("Reference not found")),
    //         Some(r) => r,
    //     };
    //     let ctx = CiteContext {
    //         reference: &refr,
    //         cite: &Cite::basic("ok"),
    //         position: Position::First,
    //         format: Markup::default(),
    //         citation_number: 1,
    //         disamb_pass: None,
    //     };
    //     let style = self.style();
    //     let mut state = IrState::new();
    //     use crate::proc::Proc;
    //     let ir = style.intermediate(self, &mut state, &ctx).0;
    //     ir.flatten(&fmt)
    //         .map(|flat| fmt.output(flat))
    //         .unwrap_or(<Markup as OutputFormat>::Output::default())
    // }

    pub fn set_references(&mut self, refs: Vec<Reference>) {
        let keys: HashSet<Atom> = refs.iter().map(|r| r.id.clone()).collect();
        for r in refs {
            self.set_reference_input(r.id.clone(), Arc::new(r));
        }
        self.set_all_keys(Arc::new(keys));
    }

    pub fn insert_reference(&mut self, refr: Reference) {
        self.set_references(vec![refr])
    }

    pub fn init_clusters(&mut self, clusters: Vec<Cluster2<Markup>>) {
        let mut cluster_ids = Vec::new();
        for cluster in clusters {
            let (cluster_id, number, cites) = cluster.split();
            let mut ids = Vec::new();
            for cite in cites.iter() {
                let cite_id = self.cite(cluster_id, Arc::new(cite.clone()));
                ids.push(cite_id);
            }
            self.set_cluster_cites(cluster_id, Arc::new(ids));
            self.set_cluster_note_number(cluster_id, number);
            cluster_ids.push(cluster_id);
        }
        self.set_cluster_ids(Arc::new(cluster_ids));
    }

    // cluster_ids is maintained manually
    // the cluster_cites relation is maintained manually

    pub fn remove_cluster(&mut self, cluster_id: ClusterId) {
        self.set_cluster_cites(cluster_id, Arc::new(Vec::new()));
        self.set_cluster_note_number(cluster_id, ClusterNumber::InText(0));
        let cluster_ids = self.cluster_ids();
        let cluster_ids: Vec<_> = (*cluster_ids)
            .iter()
            .filter(|&i| *i != cluster_id)
            .cloned()
            .collect();
        self.set_cluster_ids(Arc::new(cluster_ids));
    }

    pub fn insert_cluster(&mut self, cluster: Cluster2<Markup>) {
        let (cluster_id, number, cites) = cluster.split();
        let cluster_ids = self.cluster_ids();
        if !cluster_ids.contains(&cluster_id) {
            let mut new_cluster_ids = (*cluster_ids).clone();
            new_cluster_ids.push(cluster_id);
            self.set_cluster_ids(Arc::new(new_cluster_ids));
        }

        let mut ids = Vec::new();
        for cite in cites.iter() {
            let cite_id = self.cite(cluster_id, Arc::new(cite.clone()));
            ids.push(cite_id);
        }
        self.set_cluster_cites(cluster_id, Arc::new(ids));
        self.set_cluster_note_number(cluster_id, number);
    }

    pub fn renumber_clusters(&mut self, mappings: &[(u32, ClusterNumber)]) {
        for chunk in mappings {
            let cluster_id = chunk.0;
            let n = chunk.1;
            self.set_cluster_note_number(cluster_id, n);
        }
    }

    // Getters, because the query groups have too much exposed to publish.

    pub fn get_cite(&self, id: CiteId) -> Arc<Cite<Markup>> {
        id.lookup(self)
    }

    pub fn get_cluster(&self, cluster_id: ClusterId) -> Arc<<Markup as OutputFormat>::Output> {
        self.built_cluster(cluster_id)
    }

    pub fn get_reference(&self, ref_id: Atom) -> Option<Arc<Reference>> {
        self.reference(ref_id)
    }

    pub fn get_style(&self) -> Arc<Style> {
        self.style()
    }

    pub fn store_locales(&mut self, locales: Vec<(Lang, String)>) {
        let mut langs = (*self.locale_input_langs()).clone();
        for (lang, xml) in locales {
            langs.insert(lang.clone());
            self.set_locale_input_xml_with_durability(lang, Arc::new(xml), Durability::MEDIUM);
        }
        self.set_locale_input_langs(Arc::new(langs));
    }

    pub fn get_langs_in_use(&self) -> Vec<Lang> {
        let mut langs: HashSet<Lang> = self
            .all_keys()
            .iter()
            .filter_map(|ref_id| self.reference(ref_id.clone()))
            .filter_map(|refr| refr.language.clone())
            .collect();
        let style = self.style();
        langs.insert(style.default_locale.clone());
        langs.into_iter().collect()
    }

    pub fn has_cached_locale(&self, lang: &Lang) -> bool {
        let langs = self.locale_input_langs();
        langs.contains(lang)
    }
}
