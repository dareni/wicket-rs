use std::{env, io, path::PathBuf, sync::OnceLock};

use crate::wicket::{
    core::central::util::resource::locator::{FileResourceStreamLocator, ResourceStreamLocator},
    core::markup::ResourceStream,
};

static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn get_resource_path() -> &'static PathBuf {
    RESOURCE_DIR.get_or_init(|| {
        if cfg!(test) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources/html")
        } else if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/html")
        } else {
            let mut p = env::current_exe().expect("Failed to get exe path");
            p.pop();
            p.push("resources");
            p
        }
    })
}

/// Implement on markup component with the MarkupResourcePath eg
///       #[derive(MarkupResourcePath)]
///       struct AppComponent;
pub trait MarkupResourceLocationUtil {
    fn get_component_path(&self) -> &'static str;
    fn get_component_name(&self) -> &'static str;
    fn get_markup_type(&self) -> &'static str;
}

pub trait MarkupResourceStreamProvider {
    fn get_locator(&self) -> &dyn ResourceStreamLocator;

    ///Determine the style, variation, locale, extension here.
    fn get_markup_resource_stream<T: MarkupResourceLocationUtil>(
        &self,
        container_component: &T,
    ) -> io::Result<Box<dyn ResourceStream>> {
        let markup_path = self.get_markup_path(container_component);
        let ret = self
            .get_locator()
            .locate(&markup_path, &None, &Some("html".to_owned()));

        ret
    }

    fn get_markup_path<T: MarkupResourceLocationUtil>(&self, component: &T) -> PathBuf {
        let module_path = component.get_component_path();
        let mut component_path: PathBuf = module_path.split("::").skip(1).collect();
        let component_name = component.get_component_name();
        component_path.push(component_name);
        component_path
    }
}

/// Load the markup for a MarkupResourceProvider Component.
pub struct DefaultMarkupResourceStreamProvider {
    locator: Box<dyn ResourceStreamLocator>,
}

impl DefaultMarkupResourceStreamProvider {
    pub fn new(locator: Box<dyn ResourceStreamLocator>) -> Self {
        Self { locator }
    }

    pub fn new_default() -> DefaultMarkupResourceStreamProvider {
        println!("root::{}", get_resource_path().clone().to_str().unwrap());
        let locator = FileResourceStreamLocator::new(vec![get_resource_path().clone()]);
        Self {
            locator: Box::from(locator),
        }
    }
}

impl MarkupResourceStreamProvider for DefaultMarkupResourceStreamProvider {
    fn get_locator(&self) -> &dyn ResourceStreamLocator {
        &*self.locator
    }
}

#[cfg(test)]
mod test {
    use std::{io::Read, path::Path};

    use super::MarkupResourceLocationUtil;
    use crate::wicket::core::markup::loader::{
        DefaultMarkupResourceStreamProvider, MarkupResourceStreamProvider,
    };
    use wicket_macro::MarkupResourcePath;

    #[test]
    pub fn test_locator() {
        #[derive(MarkupResourcePath)]
        struct AppComponent;
        let tmp_comp = AppComponent {};

        assert_eq!("AppComponent", tmp_comp.get_component_name());
        assert_eq!(
            "wicket_core::wicket::core::markup::loader::test",
            tmp_comp.get_component_path()
        );

        let loader = DefaultMarkupResourceStreamProvider::new_default();
        let path = loader.get_markup_path(&tmp_comp);
        assert_eq!(
            Path::new("wicket/core/markup/loader/test/AppComponent"),
            path
        );
        let mut resource_stream = loader.get_markup_resource_stream(&tmp_comp).unwrap();
        let stream: &mut dyn Read = resource_stream.get_read();

        let mut data = String::new();
        let result = stream.read_to_string(&mut data);
        assert_eq!(25, result.unwrap());
        assert_eq!("<div>New Component</div>\n", data);
    }
}
