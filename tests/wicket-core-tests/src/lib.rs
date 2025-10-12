#[cfg(test)]
mod tests {
    use wicket_core::markup::parser::xml_pull_parser::parse;

    pub fn add(left: u64, right: u64) -> u64 {
        left + right
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
        parse();
    }
}
