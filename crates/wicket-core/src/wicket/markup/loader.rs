use std::{env, fs::File, io, path::PathBuf, sync::OnceLock};

use crate::wicket::core::util::resource::locator::{
    FileResourceStreamLocator, ResourceStreamLocator,
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
pub trait MarkupResourceProvider {
    fn get_component_path(&self) -> &'static str;
    fn get_component_name(&self) -> &'static str;
}

/// Load the markup for a MarkupResourceProvider Component.
pub struct MarkupLoader {
    locator: Box<dyn ResourceStreamLocator>,
}

impl MarkupLoader {
    pub fn new(locator: Box<dyn ResourceStreamLocator>) -> Self {
        Self { locator }
    }

    pub fn new_default() -> MarkupLoader {
        println!("root::{}", get_resource_path().clone().to_str().unwrap());
        let locator = FileResourceStreamLocator::new(vec![get_resource_path().clone()]);
        Self {
            locator: Box::from(locator),
        }
    }

    pub fn load_markup<T: MarkupResourceProvider>(&self, component: &T) -> io::Result<File> {
        let markup_path = self.get_markup_path(component);
        self.locator.locate(&markup_path)
    }

    fn get_markup_path<T: MarkupResourceProvider>(&self, component: &T) -> PathBuf {
        let module_path = component.get_component_path();
        let mut component_path: PathBuf = module_path.split("::").skip(1).collect();
        let component_name = component.get_component_name();
        component_path.push(component_name);
        component_path.set_extension("html");
        // Path::new(RESOURCE_PATH).join(component_path)
        component_path
    }
}

#[cfg(test)]
mod test {
    use std::{io::Read, path::Path};

    use crate::wicket::markup::loader::MarkupLoader;

    #[test]
    pub fn test_locator() {
        use super::MarkupResourceProvider;
        use wicket_macro::MarkupResourcePath;

        #[derive(MarkupResourcePath)]
        struct AppComponent;
        let tmp_comp = AppComponent {};

        assert_eq!("AppComponent", tmp_comp.get_component_name());
        assert_eq!(
            "wicket_core::wicket::markup::loader::test",
            tmp_comp.get_component_path()
        );

        let loader = MarkupLoader::new_default();
        let path = loader.get_markup_path(&tmp_comp);
        assert_eq!(
            Path::new("wicket/markup/loader/test/AppComponent.html"),
            path
        );
        let mut file = loader.load_markup(&tmp_comp).unwrap();
        let mut data = String::new();
        let result = file.read_to_string(&mut data);
        assert_eq!(25, result.unwrap());
        assert_eq!("<div>New Component</div>\n", data);
    }
}
