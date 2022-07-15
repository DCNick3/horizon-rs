use genco::lang::rust::Tokens;
use genco::quote;

pub struct PaddingHelper {
    number: usize,
}

impl PaddingHelper {
    pub fn new() -> Self {
        Self { number: 0 }
    }

    pub fn next_padding_name(&mut self) -> Tokens {
        let number = self.number;
        self.number += 1;
        let name = format!("_padding_{}", number);

        quote! {
            $name
        }
    }
}
