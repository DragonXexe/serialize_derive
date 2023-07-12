extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::Type;

#[proc_macro_derive(Serialize)]
pub fn serialize_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    // println!("{}", impl_serialize_macro(&ast).to_string());
    impl_serialize_macro(&ast)
}
fn impl_serialize_macro(ast: &syn::DeriveInput) -> TokenStream {
    match ast.data {
        syn::Data::Struct(_) => impl_serialize_struct_macro(ast),
        syn::Data::Enum(_) => impl_serialize_enum_macro(ast),
        syn::Data::Union(_) => todo!(),
    }
}
fn impl_serialize_struct_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let attrs = &ast.data;
    let struct_data = match attrs {
        syn::Data::Struct(r#struct) => r#struct,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };
    let fields = &struct_data.fields;
    // for field in attributes {
    //     // println!("{:?}: {:?}",field.ident, field.ty);
    // }
    let field_names: &Vec<&Option<syn::Ident>> = &fields.iter().map(|x| &x.ident).collect();
    let types: &Vec<&Type> = &fields.iter().map(|x| &x.ty).collect();
    let gen = quote! {
        impl Serialize for #name {
            fn serialize(self) -> serialr::Bytes {
                let mut bytes = serialr::Bytes::new();
                #(bytes.append(&self.#field_names.serialize());)*
                return bytes;
            }
            fn deserialize(bytes: &serialr::Bytes, mut index: usize) -> Option<Self> {
                let (#(#field_names),*): (#(#types),*);
                #(if let Some(field) = bytes.read(index) {
                    #field_names = field;
                    index += #field_names.size();
                } else {
                    return None;
                })*
                return Some(#name { #(#field_names),* });
            }
            fn size(&self) -> usize {
                #(self.#field_names.size())+*
            }

        }
    };
    gen.into()
}

fn impl_serialize_enum_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let attrs = &ast.data;
    let struct_data = match attrs {
        syn::Data::Struct(_) => todo!(),
        syn::Data::Enum(r#enum) => r#enum,
        syn::Data::Union(_) => todo!(),
    };
    let attributes = &struct_data.variants;
    // let fields: &Vec<&syn::Ident> = &attributes.iter().map(|x| &x.ident).collect();
    // let types: &Vec<&Field> = &attributes.iter().map(|x| &x.fields).collect();
    let mut variant_indicator_type = "u8";
    let mut variant_indicator_size = 1;
    if attributes.len() < 256 {
        variant_indicator_type = "u8";
        variant_indicator_size = 1;
    } else if attributes.len() < 2usize.pow(16) {
        variant_indicator_type = "u16";
        variant_indicator_size = 2;
    } else if attributes.len() < 2usize.pow(32) {
        variant_indicator_type = "u32";
        variant_indicator_size = 4;
    }

    // serialize parts
    let mut matches: Vec<String> = vec![];
    let mut i = 0;
    let mut pushes: Vec<String> = vec![];
    for variant in attributes {
        let variant_name = &variant.ident;
        let args = "_,".repeat(variant.fields.len());
        matches.push(format!("Self::{}({}) => {}", variant_name, args, i));
        let mut serialize_attr = vec![];
        let mut serialize_code = vec![];
        let mut j = 0;
        for _ in &variant.fields {
            serialize_attr.push(format!("field{}", j));
            serialize_code.push(format!("bytes.append(&field{}.serialize());", j));
            j += 1;
        }
        pushes.push(format!(
            "Self::{}({}) => {{{}}}",
            variant_name,
            serialize_attr.join(","),
            serialize_code.join("")
        ));
        i += 1;
    }
    let get_discrimenator = format!("match self {{{}}}", matches.join(",\n\t"));
    let push = format!("match self {{{}}}", pushes.join(",\n\t"));
    let serialize = format!("
    let mut bytes = serialr::Bytes::new();
    bytes.push::<{}>({});
    {}
    return bytes;",
    variant_indicator_type, get_discrimenator, push,
    );
    // deserialize parts
    let mut reverse_matches: Vec<String> = vec![];
    let mut i = 0;
    for variant in attributes {
        let mut deserialize_code: Vec<String> = vec![];
        let mut deserialize_attr = vec![];
        let mut deserialize_types: Vec<String> = vec![];
        let mut j = 0;
        for attr in &variant.fields {
            let ty = (&attr.ty).into_token_stream().to_string();
            deserialize_attr.push(format!("field{}", j));
            deserialize_types.push(format!("{}",(&attr.ty).into_token_stream().to_string()));
            deserialize_code.push(format!("if let Some(field) = bytes.read::<{}>(index) {{
                index += field.size();
                field{} = field;
            }} else {{
                return None;
            }}", ty, j));
            j += 1;
        }
        reverse_matches.push(format!("{} => {{
            let ({}): ({});
            {}
            return Some(Self::{}({}))
        }}",
        i,
        deserialize_attr.join(", "),
        deserialize_types.join(", "),
        deserialize_code.join("\n"),
        variant.ident,
        deserialize_attr.join(","),
        ));
        i += 1;
    }
    let get_discrimenator = format!("let discrimenant: {};
    if let Some(discrimenator) = bytes.read(index) {{
        discrimenant = discrimenator;
    }} else {{
        return None;
    }}
    index += 1;", variant_indicator_type);
    let deserialize = format!("{} match discrimenant {{
        {}
        _ => None
    }}",get_discrimenator, reverse_matches.join(","));
    // size parts
    let mut get_size_variant: Vec<String> = vec![];
    for variant in attributes {
        let variant_name = &variant.ident;
        let mut size_attr = vec![];
        let mut size_code = vec![];
        let mut j = 0;
        for _ in &variant.fields {
            size_attr.push(format!("field{}", j));
            size_code.push(format!("field{}.size()", j));
            j += 1;
        }
        get_size_variant.push(format!("Self::{}({}) => {}", variant_name, size_attr.join(","), size_code.join("+")));
    }
    let get_size = format!("{} + match self {{{}}}",variant_indicator_size, get_size_variant.join(",\n\t"));

    let res = format!(
        r#"impl Serialize for {} {{
            fn serialize(self) -> serialr::Bytes {{
                {}
            }}
            fn deserialize(bytes: &serialr::Bytes, mut index: usize) -> Option<Self> {{
                {}
            }}
            fn size(&self) -> usize {{
                {}
            }}
            
        }}"#,
        name,
        serialize,
        deserialize,
        get_size,
    );
    res.parse().unwrap()
}
