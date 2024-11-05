mod deserialize;
mod serialize;

extern crate proc_macro;



fn crate_name() -> proc_macro2::TokenStream {
    match proc_macro_crate::crate_name("serialr") {
        Ok(proc_macro_crate::FoundCrate::Itself) => quote::quote!(crate),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote::quote!(#ident)
        }
        Err(_) => panic!("Could not find `serialr` crate"),
    }
}

#[proc_macro_derive(Serialize)]
pub fn serialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse2(input.into()).unwrap();
    serialize::impl_serialize(&ast).into()
}
#[proc_macro_derive(Deserialize)]
pub fn deserialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse2(input.into()).unwrap();
    deserialize::impl_deserialize(&ast).into()
}


