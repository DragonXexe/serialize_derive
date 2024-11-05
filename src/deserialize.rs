

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_quote, Data, DeriveInput, GenericParam, Ident};



pub fn impl_deserialize(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let mut generics = ast.generics.clone();
    generics.params.iter_mut().for_each(|generic| {
        if let GenericParam::Type(type_param) = generic {
            type_param.bounds.push(parse_quote!(Deserialize));
        }
    });
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    
    let code = &generate_code(&ast.data);
    
    let crate_name = &super::crate_name();
    quote! {
        impl #impl_generics Deserialize for #name #ty_generics #where_clause {
            fn deserialize<B: std::io::Read>(buf: &mut B) -> Result<Self, #crate_name::SerializeError> {
                use #crate_name::SerialRead;
                #code
            }
        }
    }
}
fn generate_code(data: &Data) -> TokenStream {
    match data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                syn::Fields::Named(_fields_named) => {
                    let fields = &get_fields(&data_struct.fields);
                    quote! {
                
                        #(let #fields = buf.read_serialized()?;)*
                        return Ok(Self { #(#fields),* });
                    }
                },
                syn::Fields::Unnamed(_fields_unnamed) => {
                    let fields = &get_fields(&data_struct.fields);
                    quote! {
                
                        #(let #fields = buf.read_serialized()?;)*
                        return Ok(Self (#(#fields),*));
                    }
                },
                syn::Fields::Unit => quote! {
                    return Ok(Self);
                },
            }
            
        },
        Data::Enum(data_enum) => {
            if data_enum.variants.len() == 0 {
                return TokenStream::new();
            }
            let variants = data_enum.variants.iter().enumerate().map(|(idx, variant)| generate_variant_code(idx as u64, variant));
            let crate_name = &super::crate_name();
            quote! {
                let discriminant: u64 = buf.read_serialized::<u64>()?;
                match discriminant {
                    #(#variants),*
                    _ => Err(#crate_name::SerializeError::InvalidData),
                }
            }
        },
        Data::Union(_data_union) => unimplemented!("Doesn't support union types"),
    }
}

fn get_fields(fields: &syn::Fields) -> Vec<TokenStream> {
    match fields {
        syn::Fields::Named(fields_named) => fields_named.named.iter().map(|field| field.ident.clone().expect("should be named").into_token_stream()).collect(),
        syn::Fields::Unnamed(fields_unnamed) => (0..fields_unnamed.unnamed.len()).map(|num| Ident::new(&format!("f{num}"), Span::call_site()).into_token_stream()).collect(),
        syn::Fields::Unit => vec![],
    }
}
fn generate_variant_code(idx: u64, variant: &syn::Variant) -> TokenStream {
    let variant_name = &variant.ident;
    
    match &variant.fields {
        syn::Fields::Named(fields_named) => {
            let fields = fields_named.named.iter().map(|field| field.ident.to_token_stream());
            let fields_clone = fields.clone();
            quote! {
                #idx => {
                    #(let #fields = buf.read_serialized()?;)*
                    return Ok(Self::#variant_name {
                        #(#fields_clone),* 
                    });
                }
            }
        },
        syn::Fields::Unnamed(fields_unnamed) => {
            let field_range = 0..fields_unnamed.unnamed.len();
            let fields = field_range.into_iter().map(|field| {
                syn::Ident::new(&format!("f{field}"), Span::call_site())
            });
            let fields_clone = fields.clone();

            quote! {
                #idx => {
                    #(let #fields = buf.read_serialized()?;)*
                    return Ok(Self::#variant_name(#(#fields_clone),*));
                }
            }
        },
        syn::Fields::Unit => {
            quote! { #idx => { Ok(Self::#variant_name) } }
        },
    }
}


