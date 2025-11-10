use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(ConnectorMacro)]
pub fn connector_macro_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident; 
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    
    let _ = generics.params.iter().next().map(|param| {
        match param {
            syn::GenericParam::Type(type_param) => type_param.ident.clone(),
            _ => panic!("Traits connector type shall be generic."),
        }
    }).unwrap_or_else(|| panic!("Traits connector type shall be generic on a type."));
    
    let mut has_header_field = false;
    let fields_names =match &ast.data {
        syn::Data::Struct(data_struct) => {
            for field in &data_struct.fields {
                if let Some(ident) = &field.ident {
                    if ident == "header" {
                        has_header_field = true;
                    }
                }
            }
            has_header_field
        }
        _ => false,
    };
    if !fields_names {
        return syn::Error::new_spanned(
            name,
            "Struct must have 'header' field to derive ConnectorsTrait.",
        )
        .to_compile_error()
        .into();
    }

    let code_gen = quote! {
        //#fields_types

        impl #impl_generics ConnectorsTrait for #name #ty_generics where #where_clause {
            fn as_any(&self) -> &dyn Any {self}
            fn as_any_mut(&mut self) -> &mut dyn Any {self}
        }
    };

    code_gen.into()
}
