use std::collections::HashMap;

use crate::components::WebPage;

/// Per session page store.
pub trait PageStore {
    /// Return the version of the page.
    /// Each time a page is created its instance is stored.
    /// Each time a request mutates a page instance its version is stored.
    /// When the number of versions of a page exceeds the version number, truncate
    /// the store, so state changes to an old version reset the page history to the current
    /// version.
    fn store_page(&mut self, version: u16, page: Box<dyn WebPage>) -> u16;
    fn get_page(&mut self, page_id: u16, version: u16) -> Option<&dyn WebPage>;
}

pub struct SimplePageStore {
    pages: HashMap<u16, Vec<Box<dyn WebPage>>>,
}

impl PageStore for SimplePageStore {
    fn store_page(&mut self, version: u16, page: Box<dyn WebPage>) -> u16 {
        let page_id = page.get_markup_identity().id;

        let versions = self
            .pages
            .entry(page_id)
            .or_insert_with(|| Vec::with_capacity(1));

        let no_versions = versions.len() as u16;
        if version > (no_versions) {
            versions.truncate(no_versions as usize);
        }

        let new_version_index = versions.len() as u16;
        versions.push(page);
        new_version_index
    }

    fn get_page(&mut self, page_id: u16, version: u16) -> Option<&dyn WebPage> {
        self.pages
            .get(&page_id)?
            .get(version as usize)
            .map(|boxed| boxed.as_ref())
    }
}
