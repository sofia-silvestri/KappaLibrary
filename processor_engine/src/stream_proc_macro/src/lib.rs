use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(StreamBlockMacro)]
pub fn stream_processor_macro_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident; 
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut field_presence = vec![false; 5];
    let fields_names =match &ast.data {
        syn::Data::Struct(data_struct) => {
            for field in &data_struct.fields {
                if let Some(ident) = &field.ident {
                    match ident.to_string().as_str() {
                        "inputs" => field_presence[0] = true,
                        "outputs" => field_presence[1] = true,
                        "parameters" => field_presence[2] = true,
                        "state" => field_presence[3] = true,
                        "lock" => field_presence[4] = true,
                        _ => {}
                    }
                }
            }
            field_presence.iter().all(|&x| x) // Raccoglie i risultati di ogni 'Some(*acc)' in un nuovo Vec<bool>
        }
        _ => false,
    };

    if !fields_names {
        return syn::Error::new_spanned(
            name,
            "Struct must have 'inputs', 'outputs', 'parameters', 'state', and 'lock' fields to derive ConnectorsTrait.",
        )
        .to_compile_error()
        .into();
    }

    let code_gen = quote! {
        impl #impl_generics StreamBlockDyn for #name #ty_generics #where_clause {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
            fn check_state(&self, state: StreamingState) -> bool {
                let m_state = self.state.lock().unwrap();
                *m_state == state
            }
            fn set_state(&mut self, state: StreamingState){
                *self.state.lock().unwrap() = state;
            }
            fn get_input_list(&self) -> Vec<&str>{
                self.inputs.keys().copied().collect()
            }
            fn get_output_list(&self) -> Vec<&str>{
                self.outputs.keys().copied().collect()
            }
            fn get_parameter_list(&self) -> Vec<&str>{
                self.parameters.keys().copied().collect()
            }
        }
        impl #impl_generics StreamBlock for #name #ty_generics #where_clause {
            fn get_input_channel<V: 'static + Send + Any>(&self, key: &str) -> Result<&Sender<V>, StreamingError> {
                if let Some(container) = self.inputs.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
                    if let Some(input_container) = any_ref.downcast_ref::<Input<V>>() {
                        Ok(&input_container.sender)
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn connect<V: 'static + Send + Any + Clone>(&mut self, key: &str, sender: Sender<V>) -> Result<(), StreamingError> {
                if let Some(container) = self.outputs.get_mut(key) {
                    let any_mut: &mut dyn Any = container.as_mut();
                    if let Some(output) = any_mut.downcast_mut::<Output<V>>() {
                        output.connect(sender);
                        Ok(())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn get_parameter_value<V:'static + Send + PartialOrd + Clone>(&self, key: &str) -> Result<V, StreamingError> {
                if let Some(container) = self.parameters.get(key) {
                    let any_ref: &dyn Any = container.as_ref();
                    if let Some(param) = any_ref.downcast_ref::<Parameter<V>>() {
                        Ok(param.get_value())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
            fn set_parameter_value<V:'static + Send + PartialOrd + Clone>(&mut self, key: &str, value: V) -> Result<(), StreamingError> {
                if let Some(container) = self.parameters.get_mut(key) {
                    let any_mut: &mut dyn Any = container.as_mut();
                    if let Some(param) = any_mut.downcast_mut::<Parameter<V>>() {
                        param.set_value(value)?;
                        Ok(())
                    } else {
                        Err(StreamingError::WrongType)
                    }
                } else {
                    Err(StreamingError::InvalidInput)
                }
            }
        }
    };
    eprintln!("Generated StreamBlock implementation for {}", name);
    eprintln!("{}", code_gen);
    code_gen.into()
}