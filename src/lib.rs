extern crate proc_macro;

use proc_macro::TokenStream;
use quote::ToTokens;


#[proc_macro_derive(Serialize)]
pub fn serialize_derive(input: TokenStream) -> TokenStream {
    let genarics: Vec<String> = dbg!(get_genarics(input.clone()));
    let ast = syn::parse(dbg!(input)).unwrap();

    // Build the trait implementation
    // println!("{}", impl_serialize_macro(&ast).to_string());
    impl_serialize_macro(&ast, genarics)
}
fn impl_serialize_macro(ast: &syn::DeriveInput, genarics: Vec<String>) -> TokenStream {
    match ast.data {
        syn::Data::Struct(_) => impl_serialize_struct_macro(ast, genarics),
        syn::Data::Enum(_) => impl_serialize_enum_macro(ast, genarics),
        syn::Data::Union(_) => todo!(),
    }
}
fn impl_serialize_struct_macro(ast: &syn::DeriveInput, genarics: Vec<String>) -> TokenStream {
    let name = &ast.ident;
    let attrs = &ast.data;
    let mut is_tuple_struct = false;
    let struct_data = match attrs {
        syn::Data::Struct(r#struct) => r#struct,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };
    // generate_generics
    let genarics_impl_string: String;
    let genarics_string: String;
    if genarics.is_empty() {
        genarics_impl_string = String::new();
        genarics_string = String::new();
    } else {
        genarics_impl_string = format!("<{}>",
            genarics.iter().map(|x|format!("{}: Serialize", x)).collect::<Vec<String>>().join(", ")
        );
        genarics_string = format!("<{}>",
            genarics.join(", ")
        );
    }
    dbg!(&genarics_impl_string, &genarics_string);
    
    
    let mut fields: Vec<(String, String)> = vec![];
    let mut i = 0;
    for field in &struct_data.fields {
        if field.ident.is_some() {
            fields.push((format!("{}",field.ident.clone().unwrap()), (&field.ty).into_token_stream().to_string()));
        } else {
            is_tuple_struct = true;
            fields.push((format!("{}",i), (&field.ty).into_token_stream().to_string()));
        }
        i +=1;
    }
    
    // let fields = &struct_data.fields;
    
    // let fields: &Vec<&syn::Ident> = &attributes.iter().map(|x| &x.ident).collect();
    // let types: &Vec<&Field> = &attributes.iter().map(|x| &x.fields).collect();
    
    // serialize parts
    let mut pushes: Vec<String> = vec![];
    for field in &fields {
        pushes.push(format!("bytes.append(&self.{}.serialize());",field.0));
    }
    let serialize = format!("
    let mut bytes = serialr::Bytes::new();
    {}
    return bytes;",
    pushes.join("\n"),
    );
    // deserialize parts
    let mut matches: Vec<String> = vec![];
    let mut names: Vec<String> = vec![];
    let mut types: Vec<String> = vec![];
    for variant in &fields {
        let name: String;
        if variant.0.parse::<usize>().is_ok() {
            name = format!("field{}",variant.0);
            names.push(name.clone());
        } else {
            name = variant.0.clone();
            names.push(name.clone());
        }
        types.push(variant.1.clone());
        matches.push(format!("if let Some(field) = bytes.read(index) {{
            {} = field;
            index += {}.size();
        }} else {{
            return None;
        }}",name, name));
    }
    let return_statement: String;
    if fields.is_empty() {
        return_statement = format!("return Some(Self);")
    } else if is_tuple_struct {
        return_statement = format!("return Some(Self({}));",names.join(", "));
    } else {
        return_statement = format!("return Some(Self {{{}}});",names.join(", "))
    }
    let deserialize: String;
    if fields.is_empty() {
        deserialize = format!("return Some(Self);")
    } else {
        deserialize = format!("let ({}): ({});
        {} {}", names.join(", "), types.join(", "), matches.join(""), return_statement);
    }
    // size parts
    let mut get_size: Vec<String> = vec![];
    for variant in fields {
        get_size.push(format!("self.{}.size()",variant.0));
    }
    let get_size = format!("{}", get_size.join(" + "));

    let res = format!(
        r#"impl{} Serialize for {}{} {{
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
        genarics_impl_string,
        name,
        genarics_string,
        serialize,
        deserialize,
        get_size,
    );
    res.parse().unwrap()
}

fn impl_serialize_enum_macro(ast: &syn::DeriveInput, genarics: Vec<String>) -> TokenStream {
    let name = &ast.ident;
    let attrs = &ast.data;
    let struct_data = match attrs {
        syn::Data::Struct(_) => todo!(),
        syn::Data::Enum(r#enum) => r#enum,
        syn::Data::Union(_) => todo!(),
    };

    // generate_generics
    let genarics_impl_string: String;
    let genarics_string: String;
    if genarics.is_empty() {
        genarics_impl_string = String::new();
        genarics_string = String::new();
    } else {
        genarics_impl_string = format!("<{}>",
            genarics.iter().map(|x|format!("{}: Serialize", x)).collect::<Vec<String>>().join(", ")
        );
        genarics_string = format!("<{}>",
            genarics.join(", ")
        );
    }

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
        if variant.fields.is_empty() {
            matches.push(format!("Self::{} => {}", variant_name, i));
        } else {
            let args = "_,".repeat(variant.fields.len());
            matches.push(format!("Self::{}({}) => {}", variant_name, args, i));
        }
        let mut serialize_attr = vec![];
        let mut serialize_code = vec![];
        let mut j = 0;
        for _ in &variant.fields {
            serialize_attr.push(format!("field{}", j));
            serialize_code.push(format!("bytes.append(&field{}.serialize());", j));
            j += 1;
        }
        if variant.fields.is_empty() {
            pushes.push(format!(
                "Self::{} => ()",
                variant_name,
            ));
        } else {
            pushes.push(format!(
                "Self::{}({}) => {{{}}}",
                variant_name,
                serialize_attr.join(","),
                serialize_code.join("")
            ));
        }
        
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
        if variant.fields.is_empty() {
            reverse_matches.push(format!("{} => {{
                return Some(Self::{})
            }}",
            i,
            variant.ident,
            ));
        } else {
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
        }
        
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
        if variant.fields.is_empty() {
            get_size_variant.push(format!("Self::{} => 0", variant_name));
        } else {
            get_size_variant.push(format!("Self::{}({}) => {}", variant_name, size_attr.join(","), size_code.join("+")));
        }
    }
    let get_size = format!("{} + match self {{{}}}",variant_indicator_size, get_size_variant.join(",\n\t"));

    let res = format!(
        r#"impl{} Serialize for {}{} {{
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
        genarics_impl_string,
        name,
        genarics_string,
        serialize,
        deserialize,
        get_size,
    );
    res.parse().unwrap()
}


fn get_genarics(input: TokenStream) -> Vec<String> {
    let mut source = input.into_iter();
    while let Some(next) = source.next() {
        match next.to_string().as_str() {
            "struct" => break,
            "enum" => break,
            _ => ()
        }
    }
    source.next();
    let mut generics: Vec<String> = vec![];
    match source.next().unwrap().to_string().as_str() {
        "<" => {
            // ignore if its a lifetime
            loop {
                match source.next().unwrap().to_string().as_str() {
                    ">" => {
                        break;
                    }
                    "'" => {
                        source.next(); // consume the lifetime
                    }
                    "," => {
                        // ignore commas
                    }
                    genaric => {
                        generics.push(genaric.to_string());
                    }
                }
            }
        }
        token => {
            dbg!("no genarcis found {}",token);
        }
    }
    return generics;
}

