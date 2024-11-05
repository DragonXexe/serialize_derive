
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_quote, Data, DeriveInput, GenericParam};


pub fn impl_serialize(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let mut generics = ast.generics.clone();
    generics.params.iter_mut().for_each(|generic| {
        if let GenericParam::Type(type_param) = generic {
            type_param.bounds.push(parse_quote!(Serialize));
        }
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    
    let code = &generate_code(&ast.data);
    let crate_name = &super::crate_name();
    quote! {
        impl #impl_generics Serialize for #name #ty_generics #where_clause {
            fn serialize<B: std::io::Write>(self, buf: &mut B) -> Result<(), #crate_name::SerializeError> {
                use #crate_name::SerialWrite;
                #code
                Ok(())
            }
        }
    }
}
fn generate_code(data: &Data) -> TokenStream {
    match data {
        Data::Struct(data_struct) => {
            let fields = get_fields(&data_struct.fields);
            quote! {
                #(buf.write_serialized(self.#fields)?;)*
            }
        },
        Data::Enum(data_enum) => {
            if data_enum.variants.len() == 0 {
                return TokenStream::new();
            }
            let variants = data_enum.variants.iter().map(|variant| generate_variant_code(variant));
            
            quote! {
                let discriminant = {
                    unsafe {
                        *((&self) as *const _ as *const u64)
                    }
                };
                buf.write_serialized(discriminant)?;
                match self {
                    #(#variants),*
                }
            }
        },
        Data::Union(_data_union) => unimplemented!("Doesn't support union types"),
    }
}
fn get_fields(fields: &syn::Fields) -> Vec<TokenStream> {
    match fields {
        syn::Fields::Named(fields_named) => fields_named.named.iter().map(|field| field.ident.clone().expect("should be named").into_token_stream()).collect(),
        syn::Fields::Unnamed(fields_unnamed) => (0..fields_unnamed.unnamed.len()).map(|num| syn::Member::Unnamed(syn::Index::from(num)).into_token_stream()).collect(),
        syn::Fields::Unit => vec![],
    }
}
fn generate_variant_code(variant: &syn::Variant) -> TokenStream {
    let variant_name = &variant.ident;
    
    match &variant.fields {
        syn::Fields::Named(fields_named) => {
            let names = fields_named.named.iter().map(|field| field.ident.to_token_stream());
            let names_clone = names.clone();
            quote! {
                Self::#variant_name { #(#names),* } => {
                    #(buf.write_serialized(#names_clone)?;)*
                }
            }
        },
        syn::Fields::Unnamed(fields_unnamed) => {
            let field_range = 0..fields_unnamed.unnamed.len();
            let names = field_range.into_iter().map(|field| {
                syn::Ident::new(&format!("f{field}"), Span::call_site())
            });
            let names_clone = names.clone();

            quote! {
                Self::#variant_name(#(#names),*) => {
                    #(buf.write_serialized(#names_clone)?;)*
                }
            }
        },
        syn::Fields::Unit => quote! { Self::#variant_name => () },
    }
}

