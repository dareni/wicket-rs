pub mod parameter {

    #[derive(Default)]
    pub struct PageParameters {
        pub named_parameters: Vec<NamedPair>,
    }

    impl PageParameters {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn add(mut self, key: String, value: String) -> Self {
            self.named_parameters.push(NamedPair {
                key,
                value,
                value_type: ValueType::Manual,
            });
            self
        }

        pub fn get(&self, value_name: &str) -> Option<&NamedPair> {
            self.named_parameters
                .iter()
                .find(|param| param.key.eq_ignore_ascii_case(value_name))
        }
    }

    pub enum ValueType {
        // The named parameter is set manually in the application code.
        Manual,
        // The named parameter is read/parsed from the query string.
        QueryString,
        // The named parameter is read/parsed from the url path.
        Path,
    }

    pub struct NamedPair {
        pub key: String,
        pub value: String,
        pub value_type: ValueType,
    }
}
