use convert_case::{Case, Casing};
macro_rules! enumerate {
    // This macro takes an argument of designator `ident` and
    // creates a function named `$func_name`.
    // The `ident` designator is used for variable/function names.
    ($contracts:expr) => {
        // enum ContractEnum {
        //     Hello,
        // }
        as_item! {
            enum Test { $($body)* }
        }
    };
}

macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

macro_rules! expand_contracts {
    ($contracts:expr) => {
        let names = $contracts
            .iter()
            .map(|x| x.to_case(Case::Title))
            .collect::<Vec<String>>();
        $i
    };
}

fn test() {
    let contracts = vec!["Hello".to_string()];
    enumerate!(contracts);
}
